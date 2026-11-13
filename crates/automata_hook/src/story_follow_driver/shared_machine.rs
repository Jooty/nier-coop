use std::sync::Mutex;

use automata_game::load_gate::StoryPlace;
use automata_game::story_follow::{FollowMachine, GameRequestVerdict, StoryRequest};
use nc_hook::diagnostics::log_line;
use nc_hook::net::net_clock_ms;

static MACHINE: Mutex<Option<FollowMachine>> = Mutex::new(None);

pub(super) fn with_machine<T>(use_machine: impl FnOnce(&mut FollowMachine) -> T) -> Option<T> {
    let mut guard = MACHINE.lock().ok()?;
    guard.as_mut().map(use_machine)
}

pub(super) fn start_machine() {
    if let Ok(mut guard) = MACHINE.lock() {
        *guard = Some(FollowMachine::default());
    }
}

pub fn consult_the_follow_machine(
    request: &StoryRequest,
    current: &StoryPlace,
) -> GameRequestVerdict {
    // A detour never waits for the tick: a busy machine lets the request pass.
    let verdict = MACHINE.try_lock().ok().and_then(|mut guard| {
        guard
            .as_mut()
            .map(|machine| machine.on_game_request(request, current, net_clock_ms()))
    });
    verdict.unwrap_or_else(|| {
        log_line(&format!(
            "story: the follow machine was busy or off, the request passes unseen: {}",
            request.describe()
        ));
        GameRequestVerdict::Pass
    })
}
