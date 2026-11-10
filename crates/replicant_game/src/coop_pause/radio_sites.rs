use crate::coop_pause::{MENU_OWN_BITS, MENU_RADIO_PAUSE_BIT};

pub const GET_SLOT_ZERO_FUNCTION_RVA: u32 = 0x3B_0410;
pub const SET_SLOT_ZERO_FUNCTION_RVA: u32 = 0x3B_1370;
pub const RADIO_SLOT_ZERO_CALL_SITE_RVAS: [u32; 3] = [0x4A_AA14, 0x4A_ABA8, 0x4A_B1F8];
pub const TALK_WINDOW_CLOSE_SLOT_ZERO_RESTORE_CALL_SITE_RVA: u32 = 0x4D_F719;
pub const TALK_WINDOW_OPEN_SLOT_ZERO_SAVE_CALL_SITE_RVA: u32 = 0x4E_0409;
pub const FIND_SCENE_ROOT_FUNCTION_RVA: u32 = 0x8_6490;
pub const RADIO_CLOCK_SCENE_QUERY_CALL_SITE_RVAS: [u32; 2] = [0xC_B7A8, 0xC_BABA];
pub const PAUSE_MENU_SCENE_KIND: u8 = 2;
pub const NEAR_CALL_LENGTH: usize = 5;

const NEAR_CALL_OPCODE: u8 = 0xE8;

pub fn radio_slot_zero(slot_zero: u32, no_pause_applies: bool) -> u32 {
    if no_pause_applies {
        slot_zero & !MENU_RADIO_PAUSE_BIT
    } else {
        slot_zero
    }
}

pub fn radio_clock_scene_query_hidden(scene_kind: u8, no_pause_applies: bool) -> bool {
    scene_kind == PAUSE_MENU_SCENE_KIND && no_pause_applies
}

pub fn menu_added_bits(slot_zero_without_menu: u32, slot_zero_now: u32) -> u32 {
    slot_zero_now & !slot_zero_without_menu & MENU_OWN_BITS
}

pub fn talk_window_restored_slot_zero(
    saved: u32,
    menu_bits_added_when_saved: u32,
    pause_menu_or_prompt_open: bool,
) -> u32 {
    if pause_menu_or_prompt_open {
        saved
    } else {
        saved & !menu_bits_added_when_saved
    }
}

pub fn near_call_bytes(call_address: u64, target_address: u64) -> Option<[u8; NEAR_CALL_LENGTH]> {
    let next = call_address.checked_add(u64::try_from(NEAR_CALL_LENGTH).ok()?)?;
    let distance = i32::try_from(i128::from(target_address) - i128::from(next)).ok()?;
    let mut bytes = [NEAR_CALL_OPCODE, 0, 0, 0, 0];
    bytes[1..].copy_from_slice(&distance.to_le_bytes());
    Some(bytes)
}
