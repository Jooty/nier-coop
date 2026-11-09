use std::sync::PoisonError;

use rc_game::GameMemory;
use rc_game::fight_sync::{
    GuestArrival, GuestOpen, GuestReceived, InFlight, PHASE_SCRIPT_MANAGER_STATE_OFFSET,
    script_slot_state,
};
use rc_game::offsets::{MAP_MANAGER_ACTIVE_CONTEXT_POINTER_OFFSET, MAP_MANAGER_RVA};
use rc_game::stay_together::FollowRole;
use rc_game::story_relay::{QUEUED_CALL_ARGUMENTS_OFFSET, QUEUED_CALL_READY_STATE};
use rc_proto::{Message, StoryCall};

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::{close_menus_for_modal_entry, partner_calls_waiting, restart_signals};
use crate::diagnostics::log_line;
use crate::fight_sync::pop::{
    registered_callback_count, send_host_outs, sync_host_phase, trigger_manager,
};
use crate::fight_sync::state::{GUEST, HOST, Here, IN_FLIGHT_ARGUMENTS, SEEN_RESTARTS, here};
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::player_body::phase_load_count;
use crate::story_relay::inject::queue_call_unmarked;

pub(crate) fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let messages = net.take_relayed(|message| {
        matches!(message, Message::FightStream(_) | Message::FightWindow(_))
    });
    let restarted = phase_restarted();
    match here(build) {
        Some(here) if here.role == FollowRole::Host => host_tick(build, restarted),
        Some(here) if here.role == FollowRole::Guest => {
            guest_tick(build, here, messages, restarted);
        }
        Some(_) | None => leave_guest_stream("you are not in a session with the host"),
    }
}

fn phase_restarted() -> bool {
    let now = (phase_load_count(), restart_signals().0);
    let mut seen = SEEN_RESTARTS.lock().unwrap_or_else(PoisonError::into_inner);
    let restarted = seen.is_some_and(|before| before != now);
    *seen = Some(now);
    restarted
}

fn host_tick(build: &VerifiedBuild, restarted: bool) {
    let mut outs = sync_host_phase(build);
    let mut host = HOST.lock().unwrap_or_else(PoisonError::into_inner);
    if restarted {
        outs.extend(host.restart());
    }
    if host.is_armed() {
        drop(host);
        let count = registered_callback_count(build, trigger_manager(build));
        outs.extend(
            HOST.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .registered_callbacks(count),
        );
    } else {
        drop(host);
    }
    send_host_outs(build, outs);
}

fn guest_tick(build: &VerifiedBuild, here: Here, messages: Vec<Message>, restarted: bool) {
    let local_phase = here.local_phase;
    if restarted {
        leave_guest_stream("your game loaded or restarted the phase");
    }
    for message in messages {
        let received = GUEST
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .receive_message(message, local_phase);
        log_received(received);
    }
    if !here.partner_in_same_phase {
        leave_guest_stream("the host is not in your phase");
        return;
    }
    if !GUEST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_active_in(local_phase)
    {
        return;
    }
    requeue_a_lost_entry(build);
    let slot = script_slot_state(script_manager_state(build));
    let waiting = partner_calls_waiting();
    let entry = GUEST.lock().unwrap_or_else(PoisonError::into_inner).tick(
        slot,
        local_phase,
        waiting,
        net_clock_ms(),
    );
    let Some(entry) = entry else {
        return;
    };
    close_menus_for_modal_entry(build, entry.phase, &entry.name);
    let call = StoryCall {
        phase: entry.phase,
        pack: 0,
        function: entry.name.clone(),
        args: entry.args,
    };
    let queued = queue_call_unmarked(build, &call);
    *IN_FLIGHT_ARGUMENTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = queued;
    if queued.is_some() {
        log_line(&format!(
            "fight stream: the host's {} (seq {}) queued here",
            entry.name, entry.seq
        ));
    } else {
        GUEST
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .queued_call_lost();
        log_line(&format!(
            "fight stream: could not queue the host's {} (seq {}), tried again next tick",
            entry.name, entry.seq
        ));
    }
}

fn requeue_a_lost_entry(build: &VerifiedBuild) {
    let queued = matches!(
        GUEST
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .in_flight(),
        InFlight::Queued { .. }
    );
    let arguments = *IN_FLIGHT_ARGUMENTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let (true, Some(arguments)) = (queued, arguments) else {
        return;
    };
    let slot_state = InProcessMemory::new(build)
        .read_i32(arguments - QUEUED_CALL_ARGUMENTS_OFFSET)
        .ok();
    if slot_state == Some(QUEUED_CALL_READY_STATE) {
        return;
    }
    // The slot was freed without a matching pop (a phase restart clears every queue slot).
    let lost = GUEST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .queued_call_lost();
    if let Some(seq) = lost {
        log_line(&format!(
            "fight stream: the host's entry seq {seq} left your game's queue without running, queued again"
        ));
    }
}

fn script_manager_state(build: &VerifiedBuild) -> Option<i32> {
    let memory = InProcessMemory::new(build);
    let manager_pointer = build.module_base()
        + u64::from(MAP_MANAGER_RVA)
        + MAP_MANAGER_ACTIVE_CONTEXT_POINTER_OFFSET;
    let manager = memory
        .read_u64(manager_pointer)
        .ok()
        .filter(|&at| at != 0)?;
    memory
        .read_i32(manager + PHASE_SCRIPT_MANAGER_STATE_OFFSET)
        .ok()
}

fn leave_guest_stream(why: &str) {
    let mut guest = GUEST.lock().unwrap_or_else(PoisonError::into_inner);
    if guest.window().is_none() {
        return;
    }
    let dropped = guest.leave();
    drop(guest);
    *IN_FLIGHT_ARGUMENTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = None;
    log_line(&format!(
        "fight stream: left the host's fight ({why}), {dropped} waiting entries dropped"
    ));
}

fn log_received(received: GuestReceived) {
    match received {
        GuestReceived::Opened {
            open,
            dropped_from_before,
        } => {
            if dropped_from_before > 0 {
                log_line(&format!(
                    "fight stream: {dropped_from_before} entries of the previous fight were not run here"
                ));
            }
            match open {
                GuestOpen::Joined(window) => log_line(&format!(
                    "fight stream: the host opened fight {}-{}, your game runs the host's pops in order",
                    window.number, window.part
                )),
                GuestOpen::NotHere => log_line(
                    "fight stream: the host opened a fight in another phase, not followed here",
                ),
                GuestOpen::UnknownWindow => {
                    log_line("fight stream: the host opened a fight this build does not know");
                }
            }
        }
        GuestReceived::Closed { end_seq } => log_line(&format!(
            "fight stream: the host closed the fight after {end_seq} entries, the rest run here first"
        )),
        GuestReceived::Entry { seq, arrival } => match arrival {
            GuestArrival::Stream(rc_game::fight_sync::Arrival::Pending) => {}
            GuestArrival::Stream(other) => {
                log_line(&format!(
                    "fight stream: entry seq {seq} not kept ({other:?})"
                ));
            }
            GuestArrival::NotJoined => log_line(&format!(
                "fight stream: entry seq {seq} not run here, you did not join that fight"
            )),
        },
        GuestReceived::NotAFightMessage => {}
    }
}
