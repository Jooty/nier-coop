use crate::load_gate::{
    LOAD_DONE_REQUESTED, SCENE_LOADER_ANSWER_COMPARE_OFFSET, SCENE_LOADER_ANSWER_COMPARE_OPCODE,
    SCENE_LOADER_CODE_CHECKED_LEN, SCENE_LOADER_FUNCTION_START, SCENE_LOADER_LAST_STAGE,
    SCENE_LOADER_REQUEST_WRITE_OFFSET, SCENE_LOADER_STAGE_READ_OFFSET,
    SCENE_LOADER_STAGE_READ_OPCODE, SCENE_LOADER_STAGE_WRITE_OFFSET,
    STORE_OF_A_FOUR_BYTE_VALUE_OPCODE,
};

/// The addresses of one exe, as offsets from its image base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoaderAddresses {
    pub loader_function: u32,
    pub stage_value: u32,
    pub load_done_request_value: u32,
    pub loading_screen_answer_value: u32,
}

/// True when `code`, read at the loader's start, reads its stage, waits for the loading screen's
/// answer and ends its fourth stage by asking for the load's end at exactly the noted addresses.
pub fn loader_code_is_the_noted_one(code: &[u8], addresses: LoaderAddresses) -> bool {
    if code.len() < SCENE_LOADER_CODE_CHECKED_LEN || !code.starts_with(&SCENE_LOADER_FUNCTION_START)
    {
        return false;
    }
    let refers = |offset: usize, opcode: &[u8], bytes_after_the_address: usize, target: u32| {
        refers_to(
            code,
            addresses.loader_function,
            offset,
            opcode,
            bytes_after_the_address,
        ) == Some(target)
    };
    refers(
        SCENE_LOADER_STAGE_READ_OFFSET,
        &SCENE_LOADER_STAGE_READ_OPCODE,
        0,
        addresses.stage_value,
    ) && refers(
        SCENE_LOADER_ANSWER_COMPARE_OFFSET,
        &SCENE_LOADER_ANSWER_COMPARE_OPCODE,
        0,
        addresses.loading_screen_answer_value,
    ) && refers(
        SCENE_LOADER_REQUEST_WRITE_OFFSET,
        &STORE_OF_A_FOUR_BYTE_VALUE_OPCODE,
        4,
        addresses.load_done_request_value,
    ) && stored_value(code, SCENE_LOADER_REQUEST_WRITE_OFFSET) == Some(LOAD_DONE_REQUESTED)
        && refers(
            SCENE_LOADER_STAGE_WRITE_OFFSET,
            &STORE_OF_A_FOUR_BYTE_VALUE_OPCODE,
            4,
            addresses.stage_value,
        )
        && stored_value(code, SCENE_LOADER_STAGE_WRITE_OFFSET) == Some(SCENE_LOADER_LAST_STAGE)
}

fn refers_to(
    code: &[u8],
    function: u32,
    offset: usize,
    opcode: &[u8],
    bytes_after_the_address: usize,
) -> Option<u32> {
    let instruction = code.get(offset..)?;
    if !instruction.starts_with(opcode) {
        return None;
    }
    let distance = four_bytes(instruction, opcode.len())?;
    // The distance counts from the end of the instruction.
    let next = offset + opcode.len() + 4 + bytes_after_the_address;
    Some(
        function
            .wrapping_add(u32::try_from(next).ok()?)
            .wrapping_add(u32::from_le_bytes(distance)),
    )
}

fn stored_value(code: &[u8], offset: usize) -> Option<i32> {
    let instruction = code.get(offset..)?;
    four_bytes(instruction, STORE_OF_A_FOUR_BYTE_VALUE_OPCODE.len() + 4).map(i32::from_le_bytes)
}

fn four_bytes(bytes: &[u8], at: usize) -> Option<[u8; 4]> {
    bytes.get(at..at + 4)?.try_into().ok()
}
