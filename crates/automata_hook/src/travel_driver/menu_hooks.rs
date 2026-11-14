use std::sync::atomic::Ordering;

use automata_game::partner::PlayerEntry;
use automata_game::travel::{TeleportRequest, TravelPoint, TravelView, decide_bring_partner};
use automata_proto::Message;
use nt_hook::diagnostics::log_line;
use nt_hook::net::net_clock_ms;

use crate::actions::{Action, enqueue};
use crate::menu::TeleportHooks;
use crate::net::{SessionModeExt, ShownPlayersExt, handle};
use crate::travel_driver::driver_state::{VIEW, VIEW_ASKED_AT_MS};

const GO_TO_PARTNER_QUEUED_TEXT: &str = "asked the game to move you to the partner";
const GO_TO_PLAYER_QUEUED_TEXT: &str = "asked the game to move you to that player";
const BRING_ONLY_IN_THE_CAMPAIGN_TEXT: &str =
    "not asked: bringing a player is for the campaign only";
const BRING_PARTNER_SENT_TEXT: &str = "asked the other player's game to bring them here";

pub fn teleport_hooks() -> TeleportHooks {
    TeleportHooks {
        view,
        ask_jump,
        ask_go_to_partner,
        ask_bring_partner,
        players,
        ask_go_to_player,
    }
}

fn view() -> TravelView {
    // The game thread walks the game's point list only while the menu keeps asking.
    VIEW_ASKED_AT_MS.store(net_clock_ms().max(1), Ordering::Relaxed);
    VIEW.lock()
        .ok()
        .and_then(|view| view.clone())
        .unwrap_or_default()
}

fn ask_jump(point: TravelPoint) -> Result<(), String> {
    enqueue(Action::TravelTo(point)).map_err(|error| error.to_string())
}

fn ask_go_to_partner() -> Result<String, String> {
    enqueue(Action::GoToPartner)
        .map(|()| GO_TO_PARTNER_QUEUED_TEXT.to_owned())
        .map_err(|error| format!("not queued: {error}"))
}

fn players() -> Vec<PlayerEntry> {
    handle().map_or_else(Vec::new, |net| {
        net.update(|shared| {
            if shared.session_mode().story_rules_apply() {
                return Vec::new();
            }
            shared
                .shown_others()
                .into_iter()
                .map(|player| PlayerEntry {
                    slot: player.slot,
                    name: player.name.clone(),
                })
                .collect()
        })
    })
}

fn ask_go_to_player(slot: u8) -> Result<String, String> {
    enqueue(Action::GoToPlayer(slot))
        .map(|()| GO_TO_PLAYER_QUEUED_TEXT.to_owned())
        .map_err(|error| format!("not queued: {error}"))
}

fn ask_bring_partner() -> Result<String, String> {
    let net = handle();
    if net.is_some_and(|net| net.update(|shared| !shared.session_mode().story_rules_apply())) {
        return Err(BRING_ONLY_IN_THE_CAMPAIGN_TEXT.to_owned());
    }
    let connected = net.is_some_and(|net| net.update(|shared| shared.campaign_link_is_up()));
    let same_phase = VIEW
        .lock()
        .ok()
        .and_then(|view| view.as_ref().and_then(|view| view.same_phase_as_partner));
    decide_bring_partner(connected, same_phase)
        .map_err(|refusal| format!("not asked: {refusal}"))?;
    if let Some(net) = net {
        net.send_relayed(Message::Teleport(TeleportRequest::ComeToSender));
    }
    log_line("teleport: asked the other player's game to bring them here (a reliable message)");
    Ok(BRING_PARTNER_SENT_TEXT.to_owned())
}
