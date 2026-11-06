use std::sync::PoisonError;
use std::sync::atomic::Ordering;

use rc_game::stay_together::FollowRole;
use rc_game::{PhaseEntry, Replicant};
use rc_proto::Message;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, NetState, NetThread, net_clock_ms};
use crate::script_rolls::state::{ROLLS_MISSED, ROLLS_SENT, ROLLS_USED, STORE};

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    drain_arrived(net);
    let empty = STORE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_empty();
    if empty {
        return;
    }
    let game = Replicant::new(InProcessMemory::new(build));
    let Some(phase) = current_phase_index(&game) else {
        return;
    };
    let forgotten = forget_other_phases(phase);
    if forgotten > 0 {
        log_line(&format!(
            "script roll: phase {phase}, dropped {forgotten} rolls of an earlier phase (sent {}, used {}, missed {} so far)",
            ROLLS_SENT.load(Ordering::Relaxed),
            ROLLS_USED.load(Ordering::Relaxed),
            ROLLS_MISSED.load(Ordering::Relaxed),
        ));
    }
}

pub(super) fn drain_arrived(net: &NetThread) {
    let arrived = net.take_relayed(|message| matches!(message, Message::ScriptRoll(_)));
    if arrived.is_empty() {
        return;
    }
    let view = net.view();
    let is_guest = view.state == NetState::Connected
        && FollowRole::from_slot(view.local_slot) == FollowRole::Guest;
    if !is_guest {
        return;
    }
    let now = net_clock_ms();
    let mut store = STORE.lock().unwrap_or_else(PoisonError::into_inner);
    for message in arrived {
        if let Message::ScriptRoll(roll) = message {
            store.offer(roll, now);
        }
    }
}

pub(super) fn current_phase_index(game: &Replicant<InProcessMemory>) -> Option<u16> {
    let phase = game.current_phase().ok()?;
    PhaseEntry::index_of(&phase).and_then(|index| u16::try_from(index).ok())
}

pub(super) fn forget_other_phases(phase: u16) -> usize {
    STORE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .forget_other_phases(phase)
}
