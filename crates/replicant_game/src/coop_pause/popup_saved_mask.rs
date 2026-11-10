use crate::coop_pause::UI_DIALOG_OWN_MASK_BITS;

pub fn popup_saved_mask_after_slot_zero_write(
    ui_dialog_state: i32,
    saved: u32,
    written: u32,
) -> u32 {
    if ui_dialog_state == 0 {
        return saved;
    }
    (saved & written) | (written & !UI_DIALOG_OWN_MASK_BITS)
}
