use crate::callback_gate::{ParkedCall, ParkedKind, Released};
use crate::script_functions::script_function;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrainStep {
    Run(Released),
    Superseded { parked: ParkedCall, position: u32 },
}

pub fn plan_forced_drain(
    released: Vec<Released>,
    target: &str,
    phase_label: &str,
) -> Vec<DrainStep> {
    released
        .into_iter()
        .map(|entry| match entry {
            Released::Call { parked, position }
                if parked.kind == ParkedKind::Function
                    && parked.call.function != target
                    && !script_function(phase_label, &parked.call.function)
                        .is_some_and(|function| function.starts_scene) =>
            {
                DrainStep::Superseded { parked, position }
            }
            other => DrainStep::Run(other),
        })
        .collect()
}
