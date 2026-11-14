use std::sync::Mutex;

use automata_game::event_follow::{EventId, EventMachine, PlayVerdict, SkipVerdict};
use nt_hook::diagnostics::log_line;
use nt_hook::net::net_clock_ms;

static MACHINE: Mutex<Option<EventMachine>> = Mutex::new(None);

pub(super) fn with_machine<T>(use_machine: impl FnOnce(&mut EventMachine) -> T) -> Option<T> {
    let mut guard = MACHINE.lock().ok()?;
    guard.as_mut().map(use_machine)
}

pub(super) fn start_machine(starts_by_data_id: bool, skip_vote: bool, event_pause_together: bool) {
    if let Ok(mut guard) = MACHINE.lock() {
        *guard = Some(
            EventMachine::new(starts_by_data_id)
                .with_skip_vote(skip_vote)
                .with_event_pause(event_pause_together),
        );
    }
}

pub fn consult_the_event_machine_on_a_play(id: EventId) -> PlayVerdict {
    try_with_machine(
        |machine| machine.on_play_request(id, net_clock_ms()),
        || format!("the play of {} passes unseen", id.describe()),
    )
    .unwrap_or(PlayVerdict::Pass)
}

pub fn consult_the_event_machine_on_a_skip() -> SkipVerdict {
    try_with_machine(
        |machine| machine.on_player_skip(net_clock_ms()),
        || "the player's skip passes unseen".to_owned(),
    )
    .unwrap_or(SkipVerdict::Pass)
}

pub fn tell_the_event_machine_of_a_start(id: EventId) {
    try_with_machine(
        |machine| machine.on_start(id, net_clock_ms()),
        || format!("the start of {} was not seen", id.describe()),
    );
}

pub fn tell_the_event_machine_of_an_end(id: EventId, skipped: bool) {
    try_with_machine(
        |machine| machine.on_end(id, skipped, net_clock_ms()),
        || format!("the end of {} was not seen", id.describe()),
    );
}

fn try_with_machine<T>(
    use_machine: impl FnOnce(&mut EventMachine) -> T,
    what_is_lost: impl FnOnce() -> String,
) -> Option<T> {
    // A detour never waits for the tick: a busy machine is logged and passed by.
    let Ok(mut guard) = MACHINE.try_lock() else {
        log_line(&format!(
            "event: the event machine was busy, {}",
            what_is_lost()
        ));
        return None;
    };
    guard.as_mut().map(use_machine)
}
