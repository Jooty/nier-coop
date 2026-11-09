use std::sync::PoisonError;

use rc_game::GameMemory;
use rc_game::fight_sync::{
    GuestArrival, GuestNotice, GuestOpen, GuestReceived, GuestRestart, HostRequestOut,
    HostStreamOut, PHASE_SCRIPT_MANAGER_STATE_OFFSET, QUEUED_CALL_NAME_CAPACITY,
    QUEUED_CALL_NAME_OFFSET, QueuedCallNotice, RequestReceipt, is_our_queued_call,
    queued_call_name, script_slot_state,
};
use rc_game::offsets::{MAP_MANAGER_ACTIVE_CONTEXT_POINTER_OFFSET, MAP_MANAGER_RVA};
use rc_game::stay_together::FollowRole;
use rc_game::story_relay::QUEUED_CALL_ARGUMENTS_OFFSET;
use rc_proto::{FightRequest, FightRequestKind, Message, StoryCall};

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::{close_menus_for_modal_entry, partner_calls_waiting, restart_signals};
use crate::diagnostics::log_line;
use crate::fight_sync::object_arms::forget_object_arms;
use crate::fight_sync::pop::{
    registered_callback_count, send_host_outs, sync_host_phase, trigger_manager,
};
use crate::fight_sync::state::{GUEST, HOST, Here, SEEN_RESTARTS, here};
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::player_body::phase_load_count;
use crate::story_relay::inject::queue_call_unmarked;

pub(crate) fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let messages = net.take_relayed(|message| {
        matches!(
            message,
            Message::FightStream(_)
                | Message::FightWindow(_)
                | Message::FightRequest(_)
                | Message::FightTape(_)
        )
    });
    let restarted = phase_restarted();
    if restarted {
        // A phase restart zeroes every queue slot.
        forget_object_arms();
    }
    check_own_queued_calls(build);
    match here(build) {
        Some(here) if here.role == FollowRole::Host => host_tick(build, messages, restarted),
        Some(here) if here.role == FollowRole::Guest => {
            guest_tick(build, here, messages, restarted);
        }
        Some(_) | None => leave_guest_stream("you are not in a session with the host"),
    }
}

fn check_own_queued_calls(build: &VerifiedBuild) {
    let slot = script_slot_state(script_manager_state(build));
    let now = net_clock_ms();
    let still_queued = |name: &str, arguments: u64| queued_call_is_ours(build, name, arguments);
    let host = HOST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .check_queued_calls(slot, now, still_queued);
    let guest = GUEST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .check_queued_calls(slot, now, still_queued);
    for notice in host {
        log_queued_call_notice("fight request: the guest's", &notice);
    }
    for notice in guest {
        log_queued_call_notice("fight stream: the host's", &notice);
    }
}

fn log_queued_call_notice(whose: &str, notice: &QueuedCallNotice) {
    log_line(&match notice {
        QueuedCallNotice::LeftQueue { name } => {
            format!("{whose} {name} left your game's queue without running, queued again")
        }
        QueuedCallNotice::GaveUp { name, waited_ms } => format!(
            "{whose} {name} stayed in your game's queue for {waited_ms} ms without running, given up"
        ),
        QueuedCallNotice::EarlierGaveUp { name, waited_ms } => format!(
            "{whose} {name} from before the fight moved on stayed in your game's queue for {waited_ms} ms, no longer waited for"
        ),
    });
}

/// The queue slot of `arguments` still holds `name`, ready to pop. A slot that was freed, or
/// popped and taken by another call, reads as not ours.
pub(crate) fn queued_call_is_ours(build: &VerifiedBuild, name: &str, arguments: u64) -> bool {
    let Some(slot) = arguments.checked_sub(QUEUED_CALL_ARGUMENTS_OFFSET) else {
        return false;
    };
    let memory = InProcessMemory::new(build);
    let state = memory.read_i32(slot).ok();
    let slot_name = memory
        .read_array::<QUEUED_CALL_NAME_CAPACITY>(slot + QUEUED_CALL_NAME_OFFSET)
        .ok()
        .and_then(|bytes| queued_call_name(&bytes));
    is_our_queued_call(state, slot_name.as_deref(), name)
}

fn phase_restarted() -> bool {
    let now = (phase_load_count(), restart_signals().0);
    let mut seen = SEEN_RESTARTS.lock().unwrap_or_else(PoisonError::into_inner);
    let restarted = seen.is_some_and(|before| before != now);
    *seen = Some(now);
    restarted
}

fn host_tick(build: &VerifiedBuild, messages: Vec<Message>, restarted: bool) {
    let mut outs = sync_host_phase(build);
    let slot = script_slot_state(script_manager_state(build));
    let mut host = HOST.lock().unwrap_or_else(PoisonError::into_inner);
    outs.extend(host.script_slot(slot));
    if restarted {
        outs.extend(host.restart());
    }
    drop(host);
    for message in messages {
        if let Message::FightRequest(request) = message {
            outs.extend(receive_request(build, request));
        }
    }
    let armed = HOST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_armed();
    if armed {
        let count = registered_callback_count(build, trigger_manager(build));
        outs.extend(
            HOST.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .registered_callbacks(count),
        );
    }
    send_host_outs(build, outs);
    queue_a_guest_request(build);
}

fn receive_request(build: &VerifiedBuild, request: FightRequest) -> Vec<HostStreamOut> {
    let what = match &request.kind {
        FightRequestKind::RunPop { name, .. } => name.clone(),
        FightRequestKind::RestartPhase => "a phase restart".to_owned(),
    };
    let epoch = request.epoch;
    let out = HOST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .receive_request(request, net_clock_ms());
    match out {
        HostRequestOut::Run(RequestReceipt::Waiting) => log_line(&format!(
            "fight request: the guest asked for {what}, it runs here when the script slot is free"
        )),
        HostRequestOut::Run(RequestReceipt::AlreadyRanHere) => log_line(&format!(
            "fight request: the guest asked for {what}, your game already ran it and streamed it"
        )),
        HostRequestOut::Run(RequestReceipt::AlreadyWaiting) => log_line(&format!(
            "fight request: the guest asked for {what} again, it is already waiting"
        )),
        HostRequestOut::Run(RequestReceipt::TooManyWaiting) => log_line(&format!(
            "fight request: the guest asked for {what}, too many of its requests wait already, not run"
        )),
        HostRequestOut::Run(RequestReceipt::NotAWindowFunction) => log_line(&format!(
            "fight request: the guest asked for {what}, which is no function of this fight, not run"
        )),
        HostRequestOut::Stale => log_line(&format!(
            "fight request: the guest's {what} was for epoch {epoch}, the fight moved on, not run"
        )),
        HostRequestOut::NotInWindow => log_line(&format!(
            "fight request: the guest's {what} came outside a fight window, not run"
        )),
        HostRequestOut::RestartPhase => {
            log_line(
                "fight request: the guest restarted the fight alone, your game restarts the phase too",
            );
            if crate::rewind::rewind_now(build) {
                return HOST
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .restart();
            }
            log_line("fight request: the phase restart hook is missing, not restarted");
        }
    }
    Vec::new()
}

fn queue_a_guest_request(build: &VerifiedBuild) {
    let Some(phase) = HOST.lock().unwrap_or_else(PoisonError::into_inner).phase() else {
        return;
    };
    let slot = script_slot_state(script_manager_state(build));
    let call = HOST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .tick_requests(slot, partner_calls_waiting(), net_clock_ms());
    let Some(call) = call else {
        return;
    };
    let story_call = StoryCall {
        phase,
        pack: 0,
        function: call.name.clone(),
        args: call.args,
    };
    let queued = queue_call_unmarked(build, &story_call);
    if let Some(arguments) = queued {
        HOST.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .request_queued_at(arguments);
        log_line(&format!(
            "fight request: the guest's {} queued here",
            call.name
        ));
    } else {
        HOST.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .request_call_lost();
        log_line(&format!(
            "fight request: could not queue the guest's {}, tried again next tick",
            call.name
        ));
    }
}

fn guest_tick(build: &VerifiedBuild, here: Here, messages: Vec<Message>, restarted: bool) {
    let local_phase = here.local_phase;
    if restarted {
        let restart = GUEST
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .own_restart(net_clock_ms());
        log_restart(restart, "your game restarted the phase");
    }
    for message in messages {
        let received = GUEST
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .receive_message(message, local_phase);
        log_received(received);
    }
    let released = GUEST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .release_next_window(local_phase, net_clock_ms());
    for received in released {
        log_received(received);
    }
    send_guest_requests();
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
    let slot = script_slot_state(script_manager_state(build));
    let waiting = partner_calls_waiting();
    let entry = GUEST.lock().unwrap_or_else(PoisonError::into_inner).tick(
        slot,
        local_phase,
        waiting,
        net_clock_ms(),
    );
    send_guest_requests();
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
    if let Some(arguments) = queued {
        GUEST
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry_queued_at(arguments);
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

fn send_guest_requests() {
    let (requests, notices) = {
        let mut guest = GUEST.lock().unwrap_or_else(PoisonError::into_inner);
        (guest.take_requests(), guest.take_notices())
    };
    for notice in notices {
        log_line(&match notice {
            GuestNotice::RestartRequested { epoch } => format!(
                "fight stream: the host did not restart within the wait after your restart, asked it to (epoch {epoch})"
            ),
            GuestNotice::HostRestartedAlone { epoch } => format!(
                "fight stream: the host restarted and your game did not, the replay goes on at epoch {epoch}"
            ),
            GuestNotice::LeftWithoutHostRestart { dropped } => format!(
                "fight stream: the host never restarted after the request, left its fight ({dropped} waiting entries dropped)"
            ),
            GuestNotice::WaitedForHostEntryEnd { seq, waited_ms } => format!(
                "fight tape: the host's entry seq {seq} waited {waited_ms} ms for its end on the host before it queued here"
            ),
            GuestNotice::HostEntryEndNeverCame { seq, waited_ms } => format!(
                "fight tape: the host's end of entry seq {seq} did not come in {waited_ms} ms, queued anyway, later host values are not used"
            ),
            GuestNotice::HeldPopNeverMatched { name, epoch } => format!(
                "fight stream: your game's {name} was held and the host's stream never ran it (epoch {epoch}), nor was it sent to the host"
            ),
            GuestNotice::NextWindowWaitedTooLong { waited_ms } => format!(
                "fight stream: the host's next fight waited {waited_ms} ms for your game to follow, it goes on now"
            ),
            GuestNotice::NextWindowMessagesDropped { dropped } => format!(
                "fight stream: {dropped} messages of the host's next fight came while it waited and were not kept"
            ),
        });
    }
    let Some(net) = net::handle() else {
        return;
    };
    for request in requests {
        net.send_relayed(Message::FightRequest(request));
    }
}

fn log_restart(restart: GuestRestart, why: &str) {
    match restart {
        GuestRestart::Paired { epoch } => log_line(&format!(
            "fight stream: {why}, both games restarted, the replay goes on at the host's epoch {epoch}"
        )),
        GuestRestart::WaitingForHost => log_line(&format!(
            "fight stream: {why}, your game waits for the host's restart before it replays again"
        )),
        GuestRestart::WaitingForOwnRestart { epoch } => log_line(&format!(
            "fight stream: {why}, the replay waits for your game's own restart (epoch {epoch})"
        )),
        GuestRestart::Joined(open) => log_open(open),
        GuestRestart::SameRestart | GuestRestart::NotInFight => {}
    }
}

pub(crate) fn script_manager_state(build: &VerifiedBuild) -> Option<i32> {
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
            log_open(open);
        }
        GuestReceived::Restarted(restart) => log_restart(restart, "the host restarted the phase"),
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
        GuestReceived::Tape { seq, kept: false } => log_line(&format!(
            "fight tape: a host value for seq {seq} is for a fight or epoch you are not in, not kept"
        )),
        GuestReceived::HeldForNextWindow { first: true } => log_line(
            "fight stream: the host's next fight waits until your game can follow it (the earlier one drains, or you reach its phase)",
        ),
        GuestReceived::Tape { kept: true, .. }
        | GuestReceived::HeldForNextWindow { first: false }
        | GuestReceived::EntryEnded { .. }
        | GuestReceived::NotAFightMessage => {}
    }
}

fn log_open(open: GuestOpen) {
    match open {
        GuestOpen::Joined(window) => log_line(&format!(
            "fight stream: the host opened fight {}-{}, your game runs the host's pops in order",
            window.number, window.part
        )),
        GuestOpen::NotHere => {
            log_line("fight stream: the host opened a fight in another phase, not followed here");
        }
        GuestOpen::UnknownWindow => {
            log_line("fight stream: the host opened a fight this build does not know");
        }
    }
}
