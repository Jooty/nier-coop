use crate::partner::{
    CONTROLLER_CALLS_LEN, PROCESS_BUTTONS_FIRST_STORE, PROCESS_BUTTONS_FIRST_STORE_AT,
    PROCESS_BUTTONS_SECOND_STORE, PROCESS_BUTTONS_SECOND_STORE_AT,
};

const MOVE_CONTROLLER_TO_FIRST_ARGUMENT: [u8; 3] = [0x48, 0x8B, 0xCB];
const MOVE_REFERENCE_TO_SECOND_ARGUMENT: [u8; 3] = [0x48, 0x8B, 0xD6];
const CALL_OPCODE: u8 = 0xE8;
const COMPARE_GLOBAL_WITH_BYTE: [u8; 2] = [0x83, 0x3D];
const COMPARED_MODE: u8 = 0x05;
const PROCESS_BUTTONS_CALL_AT: usize = 3;
const STICK_UPDATE_CALL_AT: usize = 14;
const CALL_LEN: usize = 5;

/// True when the bytes are the controller update's two calls and they go to the two functions.
pub fn controller_calls_match(
    found: &[u8; CONTROLLER_CALLS_LEN],
    calls_rva: u32,
    process_buttons_rva: u32,
    stick_update_rva: u32,
) -> bool {
    found[0..3] == MOVE_CONTROLLER_TO_FIRST_ARGUMENT
        && found[8..11] == MOVE_REFERENCE_TO_SECOND_ARGUMENT
        && found[11..14] == MOVE_CONTROLLER_TO_FIRST_ARGUMENT
        && found[19..21] == COMPARE_GLOBAL_WITH_BYTE
        && found[25] == COMPARED_MODE
        && call_target(found, calls_rva, PROCESS_BUTTONS_CALL_AT) == Some(process_buttons_rva)
        && call_target(found, calls_rva, STICK_UPDATE_CALL_AT) == Some(stick_update_rva)
}

pub fn process_buttons_stores_match(function_start: &[u8]) -> bool {
    let first = function_start.get(
        PROCESS_BUTTONS_FIRST_STORE_AT
            ..PROCESS_BUTTONS_FIRST_STORE_AT + PROCESS_BUTTONS_FIRST_STORE.len(),
    );
    let second = function_start.get(
        PROCESS_BUTTONS_SECOND_STORE_AT
            ..PROCESS_BUTTONS_SECOND_STORE_AT + PROCESS_BUTTONS_SECOND_STORE.len(),
    );
    first == Some(&PROCESS_BUTTONS_FIRST_STORE[..])
        && second == Some(&PROCESS_BUTTONS_SECOND_STORE[..])
}

pub fn code_starts_with(found: &[u8], noted_start: &[u8]) -> bool {
    found.len() >= noted_start.len() && found[..noted_start.len()] == *noted_start
}

fn call_target(found: &[u8; CONTROLLER_CALLS_LEN], calls_rva: u32, call_at: usize) -> Option<u32> {
    if found[call_at] != CALL_OPCODE {
        return None;
    }
    let displacement = i32::from_le_bytes([
        found[call_at + 1],
        found[call_at + 2],
        found[call_at + 3],
        found[call_at + 4],
    ]);
    let after_call = calls_rva.checked_add((call_at + CALL_LEN) as u32)?;
    after_call.checked_add_signed(displacement)
}
