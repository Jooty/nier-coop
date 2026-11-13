use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use automata_game::story_steps::{ObservedDone, StepVerdict, StepsMachine, StoryStepsMode};
use nc_hook::net::net_clock_ms;

static MACHINE: Mutex<Option<StepsMachine>> = Mutex::new(None);
static UNSEEN_WHILE_BUSY: AtomicU32 = AtomicU32::new(0);

pub(super) fn with_machine<T>(use_machine: impl FnOnce(&mut StepsMachine) -> T) -> Option<T> {
    let mut guard = MACHINE.lock().ok()?;
    guard.as_mut().map(use_machine)
}

pub(super) fn start_machine(mode: StoryStepsMode) {
    if let Ok(mut guard) = MACHINE.lock() {
        *guard = Some(StepsMachine::new(mode));
    }
}

pub(super) fn stop_machine() {
    if let Ok(mut guard) = MACHINE.lock() {
        *guard = None;
    }
}

pub(super) fn take_unseen_while_busy() -> u32 {
    UNSEEN_WHILE_BUSY.swap(0, Ordering::AcqRel)
}

pub fn consult_the_steps_machine_on_a_done(done: &ObservedDone) -> StepVerdict {
    consult(|machine| machine.on_own_done(done, net_clock_ms()))
}

pub fn consult_the_steps_machine_on_leaving_done(id: u32) -> StepVerdict {
    consult(|machine| machine.on_leave_done(id))
}

pub fn tell_the_steps_machine_of_a_reset(id: u32) {
    consult(|machine| {
        machine.on_reset(id);
        StepVerdict::Pass
    });
}

fn consult(ask: impl FnOnce(&mut StepsMachine) -> StepVerdict) -> StepVerdict {
    // A detour never waits: a busy machine lets the game's state change pass.
    let Ok(mut guard) = MACHINE.try_lock() else {
        UNSEEN_WHILE_BUSY.fetch_add(1, Ordering::AcqRel);
        return StepVerdict::Pass;
    };
    guard.as_mut().map_or(StepVerdict::Pass, ask)
}
