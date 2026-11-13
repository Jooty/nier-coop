use std::panic;
use std::sync::OnceLock;

use automata_game::address::{
    GIVE_ITEM_FUNCTION, GameAddress, INVENTORY_ADD_FUNCTION, ITEM_NAME_TEXT_GROUP_FUNCTION,
    ITEM_NAME_TEXT_ID_FUNCTION, UI_TEXT_LOOKUP_FUNCTION,
};
use automata_game::build::variant_of_image_size;
use automata_game::co_op_log::UI_TEXT_LOOKUP_FUNCTION_START;
use automata_game::partner::code_starts_with;
use automata_game::partner_hud::shown_name;
use automata_game::pickups::{
    GIVE_ITEM_FUNCTION_START, GivenItem, INVENTORY_ADD_FUNCTION_START,
    ITEM_NAME_TEXT_GROUP_FUNCTION_START, ITEM_NAME_TEXT_ID_FUNCTION_BYTES_AFTER_THE_COOKIE_LOAD,
    ITEM_NAME_TEXT_ID_FUNCTION_BYTES_AFTER_THE_COOKIE_LOAD_OFFSET,
    ITEM_NAME_TEXT_ID_FUNCTION_START, ItemPickup, heard_pickup, heard_pickup_line,
    is_key_item_kind, local_pickup_line, merge_given_items,
};
use automata_proto::Message;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::hud::post_line;
use crate::net::handle;
use crate::pickups_driver::give_item_hook::{install_give_item_hook, take_given_item_ids};

static CONTEXT: OnceLock<PickupsContext> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
struct PickupsContext {
    calls: GameCalls,
    names_are_trusted: bool,
    inventory_add_is_trusted: bool,
}

/// Hooks the give-item call; before the HUD install, which hooks the UI text lookup this checks.
pub fn install(build: VerifiedBuild) {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        log_line("pickups: image size matches no known exe, pickups are not told");
        return;
    };
    let calls = GameCalls::new(build, variant);
    let give_matches = starts_with(calls, GIVE_ITEM_FUNCTION, &GIVE_ITEM_FUNCTION_START, 0);
    let names_are_trusted = starts_with(
        calls,
        ITEM_NAME_TEXT_ID_FUNCTION,
        &ITEM_NAME_TEXT_ID_FUNCTION_START,
        0,
    ) && starts_with(
        calls,
        ITEM_NAME_TEXT_ID_FUNCTION,
        &ITEM_NAME_TEXT_ID_FUNCTION_BYTES_AFTER_THE_COOKIE_LOAD,
        ITEM_NAME_TEXT_ID_FUNCTION_BYTES_AFTER_THE_COOKIE_LOAD_OFFSET,
    ) && starts_with(
        calls,
        ITEM_NAME_TEXT_GROUP_FUNCTION,
        &ITEM_NAME_TEXT_GROUP_FUNCTION_START,
        0,
    ) && starts_with(
        calls,
        UI_TEXT_LOOKUP_FUNCTION,
        &UI_TEXT_LOOKUP_FUNCTION_START,
        0,
    );
    let inventory_add_is_trusted = starts_with(
        calls,
        INVENTORY_ADD_FUNCTION,
        &INVENTORY_ADD_FUNCTION_START,
        0,
    );
    let hooked = give_matches && install_give_item_hook(calls);
    log_line(&format!(
        "pickups: give-item call hooked: {hooked} (start bytes match: {give_matches}), item names read from the game: {names_are_trusted}, key items given to this player by the inventory add: {inventory_add_is_trusted}"
    ));
    let _ = CONTEXT.set(PickupsContext {
        calls,
        names_are_trusted,
        inventory_add_is_trusted,
    });
}

/// Game thread, once a frame: tells the other players this player's pickups and logs theirs.
pub fn tick() {
    let Some(context) = CONTEXT.get() else {
        return;
    };
    let _ = panic::catch_unwind(|| {
        context.send_local_pickups();
        context.hear_pickups();
    });
}

impl PickupsContext {
    fn send_local_pickups(&self) {
        let given_ids = take_given_item_ids();
        if given_ids.is_empty() {
            return;
        }
        let Some(net) = handle() else {
            return;
        };
        let given: Vec<GivenItem> = given_ids
            .iter()
            .map(|item_id| GivenItem {
                item_id: *item_id,
                key_item: i32::try_from(*item_id)
                    .ok()
                    .and_then(|item_id| self.calls.item_kind(item_id))
                    .is_some_and(is_key_item_kind),
            })
            .collect();
        let anyone_else = net.view().others().next().is_some();
        for pickup in merge_given_items(&given) {
            if !anyone_else {
                log_line(&format!(
                    "pickups: picked up item id {} x{} (key item {}), nobody else to tell",
                    pickup.item_id, pickup.count, pickup.key_item
                ));
                continue;
            }
            net.send_relayed(Message::Pickup(pickup));
            log_line(&format!(
                "pickups: sent item id {} x{} (key item {}) to the other players",
                pickup.item_id, pickup.count, pickup.key_item
            ));
            if let Some(line) = local_pickup_line(self.name_of(pickup).as_deref(), pickup) {
                post_line(&line);
            }
        }
    }

    fn hear_pickups(&self) {
        let Some(net) = handle() else {
            return;
        };
        let arrived = net.take_relayed_from(|message| matches!(message, Message::Pickup(_)));
        if arrived.is_empty() {
            return;
        }
        let view = net.view();
        for (sender, message) in arrived {
            let Message::Pickup(pickup) = message else {
                continue;
            };
            let name = shown_name(
                view.player(sender)
                    .map_or("", |player| player.name.as_str()),
            );
            let Some(heard) = heard_pickup(sender, view.local_slot, &name, pickup) else {
                log_line(&format!(
                    "pickups: ignored item id {} x{} from slot {sender} (our own or out of range)",
                    pickup.item_id, pickup.count
                ));
                continue;
            };
            let granted = heard.grant_item_id.map(|item_id| {
                self.inventory_add_is_trusted
                    && i32::try_from(item_id)
                        .is_ok_and(|item_id| self.calls.add_one_to_inventory(item_id))
            });
            let line = heard_pickup_line(
                &heard.sender_name,
                self.name_of(heard.pickup).as_deref(),
                heard.pickup,
                granted,
            );
            post_line(&line);
            log_line(&format!(
                "pickups: heard item id {} x{} from slot {sender} (key item {}, given to this player too: {granted:?}); the item stays in this world",
                pickup.item_id, pickup.count, pickup.key_item
            ));
        }
    }

    fn name_of(&self, pickup: ItemPickup) -> Option<String> {
        if !self.names_are_trusted {
            return None;
        }
        self.calls.item_name(i32::try_from(pickup.item_id).ok()?)
    }
}

fn starts_with(calls: GameCalls, address: GameAddress, noted: &[u8], offset: usize) -> bool {
    calls
        .read_bytes(address, offset + noted.len())
        .is_ok_and(|found| code_starts_with(&found[offset..], noted))
}
