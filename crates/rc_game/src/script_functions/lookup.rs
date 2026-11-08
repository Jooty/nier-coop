use crate::script_functions::SCRIPT_FUNCTIONS;

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

pub fn is_callee_of(phase: &str, callee: &str, caller: &str) -> bool {
    script_function(phase, caller).is_some_and(|function| function.callees.contains(&callee))
}

pub fn shows_modal_ui(phase: &str, name: &str) -> bool {
    script_function(phase, name).is_some_and(|function| function.shows_modal_ui)
}
