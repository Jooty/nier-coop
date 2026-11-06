use core::ffi::c_void;
use std::sync::{LazyLock, Mutex, PoisonError};

use rc_game::door_sync::{DoorRegistry, GuestUseWindow, action_of_message};
use rc_game::offsets::DOOR_USE_MESSAGE_ID;
use rc_game::stay_together::FollowRole;
use rc_proto::{DoorAction, DoorChange, MAX_DOOR_LAYOUT_ID, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::door_sync::calls::{applying_from_network, layout_id_of};
use crate::net;

pub(super) static REGISTRY: LazyLock<Mutex<DoorRegistry>> = LazyLock::new(Mutex::default);
pub(super) static GUEST_USES: LazyLock<Mutex<GuestUseWindow>> = LazyLock::new(Mutex::default);

pub fn observe(build: &VerifiedBuild, door: *mut c_void, message: u32, argument: i64) {
    if applying_from_network() {
        return;
    }
    let action = action_of_message(message);
    if action.is_none() && message != DOOR_USE_MESSAGE_ID {
        return;
    }
    let Ok(layout_id) = u16::try_from(layout_id_of(build, door)) else {
        return;
    };
    if layout_id > MAX_DOOR_LAYOUT_ID {
        return;
    }
    REGISTRY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note(layout_id);
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let role = FollowRole::from_slot(view.local_slot);
    let Some(action) = action else {
        if role == FollowRole::Guest {
            GUEST_USES
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .note_use(layout_id, super::tick::now_ms());
        }
        return;
    };
    let change = DoorChange {
        layout_id,
        action,
        flag: argument != 0,
    };
    match role {
        FollowRole::Host if view.partner_in_play() => {
            net.send_relayed(Message::DoorChange(change));
            log_line(&format!(
                "doors: host sent {action:?} {} for door {layout_id}",
                change.flag
            ));
        }
        FollowRole::Guest if guest_open_follows_a_use(change) => {
            net.send_relayed(Message::DoorRequest(change));
            log_line(&format!(
                "doors: guest asked the host to open door {layout_id}"
            ));
        }
        FollowRole::Guest => log_line(&format!(
            "doors: guest's own {action:?} {} on door {layout_id} stays local",
            change.flag
        )),
        _ => {}
    }
}

fn guest_open_follows_a_use(change: DoorChange) -> bool {
    change.action == DoorAction::Open
        && GUEST_USES
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .was_used_recently(change.layout_id, super::tick::now_ms())
}
