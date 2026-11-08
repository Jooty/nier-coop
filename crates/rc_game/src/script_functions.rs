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
pub use ran::{FunctionsRan, LateCopy, late_copy_verdict};
pub use table::SCRIPT_FUNCTIONS;
pub use trace::{
    debug_print_args_len, debug_print_tag_range, debug_print_text_range, traced_function_name,
    traced_function_text, traced_function_text_from_parts,
};

pub const DEBUG_PRINT_NATIVE_RVA: u32 = 0x3_CC50;
pub const DEBUG_PRINT_TEXT_ARG_INDEX: usize = 0;
pub const MAX_RAN_FUNCTIONS: usize = 1024;
pub const TRACE_ARROW: &str = "->";
pub const TRACE_CALL_SUFFIX: &str = "()";
