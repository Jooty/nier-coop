mod arrival;
mod drain;
mod fill_in;
mod lookup;
mod ran;
mod reach;
mod relay;
mod table;
mod trace;

pub use arrival::{
    FunctionArrival, FunctionArrivalFacts, FunctionDrop, function_arrival, judge_function,
};
pub use drain::{DrainStep, plan_forced_drain};
pub use fill_in::{
    FillInArrival, FillInCopy, FillInFlight, FillInNotice, FillInReach, FillInRelay, fill_in_reach,
};
pub use lookup::{
    ScriptFunction, is_called_only_from_local_state, is_callee_of, relayed_function,
    script_function, shows_modal_ui,
};
pub use ran::{
    ClearCause, FunctionsRan, LateCopy, PhaseLoadWatch, is_restart_function, late_copy_verdict,
};
pub use reach::FUNCTIONS_CALLED_ONLY_FROM_LOCAL_STATE;
pub use relay::{TracedRelay, traced_relay};
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
pub const FILL_IN_GRACE_MS: u64 = 3_000;
pub const FILL_IN_QUEUED_CALL_POP_EXPECTED_MS: u64 = 5_000;
