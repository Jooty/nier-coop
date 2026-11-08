mod arrival;
mod drain;
mod lookup;
mod ran;
mod table;
mod trace;

pub use arrival::{
    FunctionArrival, FunctionArrivalFacts, FunctionDrop, function_arrival, judge_function,
};
pub use drain::{DrainStep, plan_forced_drain};
pub use lookup::{ScriptFunction, is_callee_of, script_function, shows_modal_ui};
pub use ran::FunctionsRan;
pub use table::SCRIPT_FUNCTIONS;
pub use trace::{debug_print_args_len, traced_function_name, traced_function_text};

pub const DEBUG_PRINT_NATIVE_RVA: u32 = 0x3_CC50;
pub const DEBUG_PRINT_TEXT_ARG_INDEX: usize = 0;
pub const MAX_RAN_FUNCTIONS: usize = 1024;
pub const TRACE_ARROW: &str = "->";
pub const TRACE_CALL_SUFFIX: &str = "()";
