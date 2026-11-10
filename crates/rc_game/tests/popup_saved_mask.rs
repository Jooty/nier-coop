use rc_game::coop_pause::{UI_DIALOG_OWN_MASK_BITS, popup_saved_mask_after_slot_zero_write};

const POPUP_OPEN: i32 = 1;

fn after_write(saved: u32, written: u32) -> u32 {
    popup_saved_mask_after_slot_zero_write(POPUP_OPEN, saved, written)
}

#[test]
fn talks_last_step_under_the_popup_releases_the_kept_input_lock_mod_view() {
    assert_eq!(after_write(0x80, 0x0), 0x0);
}

#[test]
fn talks_last_step_under_the_popup_releases_the_kept_input_lock_unmodded_view() {
    assert_eq!(after_write(0x80, 0x1F), 0x0);
}

#[test]
fn the_popups_own_open_write_leaves_the_saved_mask_alone() {
    for saved in [0x0, 0x80, 0xA0, 0x481] {
        assert_eq!(after_write(saved, saved | UI_DIALOG_OWN_MASK_BITS), saved);
    }
}

#[test]
fn the_popups_own_close_write_leaves_the_saved_mask_alone() {
    for saved in [0x0, 0x80, 0xA0, 0x481] {
        assert_eq!(after_write(saved, saved), saved);
    }
}

#[test]
fn a_raw_derived_write_keeps_the_snapshot() {
    assert_eq!(after_write(0x481, 0x4BF), 0x481);
}

#[test]
fn a_pause_menu_opening_and_closing_over_the_popup_keeps_what_the_popup_held() {
    let opened = after_write(0x80, 0x80 | 0xBF | 0x19F);
    assert_eq!(opened, 0x180);
    assert_eq!(after_write(opened, 0xBF), 0x80);
}

#[test]
fn a_bit_outside_the_popups_own_bits_is_taken_over() {
    assert_eq!(after_write(0x0, 0x100), 0x100);
}

#[test]
fn a_bit_inside_the_popups_own_bits_is_not_taken_over() {
    assert_eq!(after_write(0x0, 0xA0), 0x0);
}

#[test]
fn no_open_popup_leaves_the_saved_mask_alone() {
    assert_eq!(popup_saved_mask_after_slot_zero_write(0, 0x80, 0x0), 0x80);
    assert_eq!(popup_saved_mask_after_slot_zero_write(0, 0x0, 0x100), 0x0);
}

#[test]
fn the_owners_run_ends_with_slot_zero_clear_after_both_popups() {
    let saved_by_first_popup = 0x80;
    let after_talks_last_step = after_write(saved_by_first_popup, 0x0);
    assert_eq!(after_talks_last_step, 0x0);

    let restored_by_first_close = after_talks_last_step;
    assert_eq!(restored_by_first_close, 0x0);

    let saved_by_second_popup = restored_by_first_close;
    let second_open_write = saved_by_second_popup | UI_DIALOG_OWN_MASK_BITS;
    let still_saved = after_write(saved_by_second_popup, second_open_write);
    assert_eq!(still_saved, 0x0);
    assert_eq!(after_write(still_saved, still_saved), 0x0);
}
