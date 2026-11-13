use replicant_game::callback_gate::GateRole;
use replicant_game::shared_events::phase_name;
use replicant_game::shared_sequence::is_led_by_the_host;

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::callback_gate::{gate_role, note_started_from_partner};
use crate::cutscene_skip::scene_follow::local_phase;
use crate::cutscene_skip::shared_functions::{
    function_ran_here, note_function_ran, send_to_partner,
};
use crate::diagnostics::log_line;
use crate::net;

/// A step of a shared sequence that only the host's guide reaches, at the moment a game pops it.
/// The host sends it to the guest; the guest runs the host's copy and drops its own later copy.
/// Returns true when this pop is dropped.
pub(crate) fn judge_led_step(build: &VerifiedBuild, name: &str, queued_by_the_mod: bool) -> bool {
    let Some(phase) = local_phase(build) else {
        return false;
    };
    let Some(label) = phase_name(phase) else {
        return false;
    };
    if !is_led_by_the_host(label, name) || crate::fight_sync::covers_phase(phase) {
        return false;
    }
    let Some(net) = net::handle() else {
        return false;
    };
    match gate_role(&net.view()) {
        Some(GateRole::Host) => {
            if note_function_ran(phase, name) {
                log_line(&format!(
                    "shared sequence: your game reached {name}, a step only the host's guide reaches"
                ));
                send_to_partner(phase, name);
            }
            false
        }
        Some(GateRole::Guest) if queued_by_the_mod => {
            note_function_ran(phase, name);
            note_started_from_partner(name, phase);
            log_line(&format!(
                "shared sequence: {name} runs here from the host's copy, you are in the host's sequence"
            ));
            false
        }
        Some(GateRole::Guest) if function_ran_here(phase, name) => {
            log_line(&format!(
                "shared sequence: your game reached {name} after the host's copy ran here, not run a second time"
            ));
            true
        }
        Some(GateRole::Guest) => {
            note_function_ran(phase, name);
            false
        }
        None => false,
    }
}
