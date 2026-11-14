use std::sync::{Mutex, PoisonError};

use replicant_game::callback_gate::GateRole;
use replicant_game::shared_events::phase_name;
use replicant_game::shared_sequence::{LedStepPop, is_led_by_the_host, led_step_pop};

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::callback_gate::{gate_role, note_started_from_partner};
use crate::cutscene_skip::scene_follow::local_phase;
use crate::cutscene_skip::shared_functions::{
    function_ran_here, note_function_ran, send_to_partner,
};
use crate::diagnostics::log_line;
use crate::net;

static HOSTS_COPY_RUNNING: Mutex<Option<String>> = Mutex::new(None);

/// A step of a shared sequence that only the host's guide reaches, at the moment a game pops it.
/// The host sends it to the guest; the guest runs the host's copy once and drops any other copy.
/// `None` when the pop is not such a step.
pub(crate) fn judge_led_step(
    build: &VerifiedBuild,
    name: &str,
    queued_by_the_mod: bool,
) -> Option<LedStepPop> {
    let phase = local_phase(build)?;
    let label = phase_name(phase)?;
    if !is_led_by_the_host(label, name) || crate::fight_sync::covers_phase(phase) {
        return None;
    }
    let net = net::handle()?;
    let host = match gate_role(&net.view())? {
        GateRole::Host => true,
        GateRole::Guest => false,
    };
    let pop = led_step_pop(host, queued_by_the_mod, function_ran_here(phase, name));
    if !pop.passes_the_duplicate_checks() && !pop.drops() {
        *HOSTS_COPY_RUNNING
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = None;
    }
    match pop {
        LedStepPop::HostRunsAndSends => {
            note_function_ran(phase, name);
            log_line(&format!(
                "shared sequence: your game reached {name}, a step only the host's guide reaches"
            ));
            send_to_partner(phase, name);
        }
        LedStepPop::HostRunsAgain => {}
        LedStepPop::GuestRunsTheHostsCopy => {
            note_function_ran(phase, name);
            note_started_from_partner(name, phase);
            *HOSTS_COPY_RUNNING
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Some(name.to_owned());
            log_line(&format!(
                "shared sequence: {name} runs here from the host's copy, you are in the host's sequence"
            ));
        }
        LedStepPop::GuestDropsASecondCopyOfTheHosts => log_line(&format!(
            "shared sequence: a second copy of the host's {name} came (both relays carry it), the first already ran here"
        )),
        LedStepPop::GuestDropsItsOwnCopy => log_line(&format!(
            "shared sequence: your game reached {name} after the host's copy ran here, not run a second time"
        )),
        LedStepPop::GuestRunsItsOwn => {
            note_function_ran(phase, name);
        }
    }
    Some(pop)
}

/// The script call `call` the game runs now is the host's copy of a step of its sequence.
pub(crate) fn runs_the_hosts_step_copy(call: &str) -> bool {
    HOSTS_COPY_RUNNING
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_deref()
        == Some(call)
}

/// The script call `call` is a step of a shared sequence only the host's guide reaches.
pub(crate) fn is_led_step_here(build: &VerifiedBuild, call: &str) -> bool {
    local_phase(build)
        .and_then(phase_name)
        .is_some_and(|label| is_led_by_the_host(label, call))
}
