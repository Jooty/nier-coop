use crate::script_functions::{
    FUNCTIONS_CALLED_ONLY_FROM_LOCAL_STATE, MODAL_UI_FUNCTIONS, SCRIPT_FUNCTIONS,
};
use crate::shared_sequence::is_led_by_the_host;
use crate::story_calls::is_event_id_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptFunction {
    pub starts_scene: bool,
    pub shows_modal_ui: bool,
    pub callees: &'static [&'static str],
}

pub fn script_function(phase: &str, name: &str) -> Option<ScriptFunction> {
    let index = SCRIPT_FUNCTIONS
        .binary_search_by(|(table_phase, table_name, ..)| {
            (*table_phase, *table_name).cmp(&(phase, name))
        })
        .ok()?;
    let (_, _, starts_scene, shows_modal_ui, callees) = SCRIPT_FUNCTIONS[index];
    Some(ScriptFunction {
        starts_scene,
        shows_modal_ui,
        callees,
    })
}

/// A function the shared function relay carries: one of the phase's shared functions, or an
/// event callback outside the table (the sender relays those only when its engine popped them,
/// e.g. `EID_0200_a0037g_EnemyCount`), or a step only the host's guide reaches. Such a callback
/// is taken as a plain step with no scene.
pub fn relayed_function(phase: &str, name: &str) -> Option<ScriptFunction> {
    script_function(phase, name).or_else(|| {
        (is_event_id_name(name) || is_led_by_the_host(phase, name)).then_some(ScriptFunction {
            starts_scene: false,
            shows_modal_ui: false,
            callees: &[],
        })
    })
}

pub fn is_called_only_from_local_state(phase: &str, name: &str) -> bool {
    FUNCTIONS_CALLED_ONLY_FROM_LOCAL_STATE
        .binary_search(&(phase, name))
        .is_ok()
}

pub fn is_callee_of(phase: &str, callee: &str, caller: &str) -> bool {
    script_function(phase, caller).is_some_and(|function| function.callees.contains(&callee))
}

pub fn shows_modal_ui(phase: &str, name: &str) -> bool {
    script_function(phase, name).is_some_and(|function| function.shows_modal_ui)
        || MODAL_UI_FUNCTIONS.binary_search(&(phase, name)).is_ok()
}
