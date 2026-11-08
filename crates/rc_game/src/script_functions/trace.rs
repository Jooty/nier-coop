use crate::dummy_hits::{native_text_arg, native_text_args_len};
use crate::script_functions::{DEBUG_PRINT_TEXT_ARG_INDEX, TRACE_ARROW, TRACE_CALL_SUFFIX};

pub const fn debug_print_args_len() -> usize {
    native_text_args_len(DEBUG_PRINT_TEXT_ARG_INDEX)
}

pub fn traced_function_text(args: &[u8]) -> Option<String> {
    native_text_arg(args, DEBUG_PRINT_TEXT_ARG_INDEX).ok()
}

pub fn traced_function_name(text: &str) -> Option<&str> {
    let (after_arrow, has_arrow) = match text.rsplit_once(TRACE_ARROW) {
        Some((_, name)) => (name, true),
        None => (text, false),
    };
    let (name, has_call_suffix) = match after_arrow.strip_suffix(TRACE_CALL_SUFFIX) {
        Some(name) => (name, true),
        None => (after_arrow, false),
    };
    let is_name = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    (is_name && (has_arrow || has_call_suffix)).then_some(name)
}
