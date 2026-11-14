use std::sync::{Mutex, PoisonError};

use replicant_game::own_ride::{RideChange, RideWatch, is_a_ride_function};
use replicant_game::{PhaseEntry, Replicant};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::last_pop;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;

static WATCH: Mutex<RideWatch> = Mutex::new(RideWatch::new());

/// A function this game ran (it printed its name): a ride function starts or ends this
/// player's own ride.
pub(crate) fn ride_function_ran(build: &VerifiedBuild, name: &str) {
    if !is_a_ride_function(name) {
        return;
    }
    let game = Replicant::new(InProcessMemory::new(build));
    let phase = game
        .current_phase()
        .ok()
        .as_deref()
        .and_then(PhaseEntry::index_of)
        .and_then(|index| u16::try_from(index).ok());
    let change = {
        let mut watch = WATCH.lock().unwrap_or_else(PoisonError::into_inner);
        watch.enter_phase(phase);
        watch.ran(name, net_clock_ms())
    };
    let Some(change) = change else {
        return;
    };
    let body = game.position().map_or_else(
        |_| "unreadable".to_owned(),
        |at| format!("({:.1}, {:.1}, {:.1})", at.x, at.y, at.z),
    );
    match change {
        RideChange::Started => {
            let from = last_pop().map_or_else(|| "none".to_owned(), |(call, _)| call);
            log_line(&format!(
                "ride: your own ride started here with {name} (from {from}), your body at {body}; the partner's game does not start it"
            ));
        }
        RideChange::Ended { by, ridden_ms } => {
            let ridden = ridden_ms.map_or_else(
                || "its start was not seen here".to_owned(),
                |ms| format!("after {ms} ms"),
            );
            log_line(&format!(
                "ride: your own ride ended here with {by} ({ridden}), your body at {body}"
            ));
        }
    }
}

pub(crate) fn own_ride_moves_the_player() -> bool {
    WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .moves_the_player(net_clock_ms())
}
