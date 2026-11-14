use crate::load_gate::loader_code::refers_to;
use crate::load_gate::{
    STILL_LOADING_TEST_CODE_CHECKED_LEN, STILL_LOADING_TEST_FUNCTION_END,
    STILL_LOADING_TEST_FUNCTION_END_OFFSET, STILL_LOADING_TEST_FUNCTION_START,
    STILL_LOADING_TEST_SCENE_STATE_COMPARE_OFFSET, STILL_LOADING_TEST_SCENE_STATE_COMPARE_OPCODE,
};

/// True when `code`, read at the start of the loading screen's "is a load still going" test, has
/// the noted start and end and compares the scene state at exactly the noted address. All
/// addresses are offsets from the image base.
pub fn still_loading_test_code_is_the_noted_one(
    code: &[u8],
    test_function: u32,
    scene_state_value: u32,
) -> bool {
    let end = code.get(STILL_LOADING_TEST_FUNCTION_END_OFFSET..STILL_LOADING_TEST_CODE_CHECKED_LEN);
    code.starts_with(&STILL_LOADING_TEST_FUNCTION_START)
        && end == Some(&STILL_LOADING_TEST_FUNCTION_END[..])
        // The compare ends with the one-byte value the scene state is compared with.
        && refers_to(
            code,
            test_function,
            STILL_LOADING_TEST_SCENE_STATE_COMPARE_OFFSET,
            &STILL_LOADING_TEST_SCENE_STATE_COMPARE_OPCODE,
            1,
        ) == Some(scene_state_value)
}
