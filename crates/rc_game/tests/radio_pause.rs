use rc_game::coop_pause::{
    GET_SLOT_ZERO_FUNCTION_RVA, RADIO_SLOT_ZERO_CALL_SITE_RVAS, SET_SLOT_ZERO_FUNCTION_RVA,
    TALK_WINDOW_CLOSE_SLOT_ZERO_RESTORE_CALL_SITE_RVA, TALK_WINDOW_OPEN_SLOT_ZERO_SAVE_CALL_SITE_RVA,
    menu_added_bits, near_call_bytes, radio_slot_zero,
    talk_window_restored_slot_zero,
};

const IMAGE_BASE: u64 = 0x1_4000_0000;

fn installed_exe_call_bytes(site_rva: u32, target_rva: u32) -> [u8; 5] {
    near_call_bytes(
        IMAGE_BASE + u64::from(site_rva),
        IMAGE_BASE + u64::from(target_rva),
    )
    .unwrap()
}

#[test]
fn the_radio_sees_no_menu_pause_bit_only_while_no_pause_applies() {
    assert_eq!(radio_slot_zero(0x19F, true), 0x9F);
    assert_eq!(radio_slot_zero(0x19F, false), 0x19F);
    assert_eq!(radio_slot_zero(0x1BF, true), 0xBF);
    assert_eq!(radio_slot_zero(0x40, true), 0x40);
    assert_eq!(radio_slot_zero(0, true), 0);
}

#[test]
fn the_three_radio_call_sites_match_the_installed_exe_bytes() {
    let expected: [[u8; 5]; 3] = [
        [0xE8, 0xF7, 0x59, 0xF0, 0xFF],
        [0xE8, 0x63, 0x58, 0xF0, 0xFF],
        [0xE8, 0x13, 0x52, 0xF0, 0xFF],
    ];
    for (site, bytes) in RADIO_SLOT_ZERO_CALL_SITE_RVAS.into_iter().zip(expected) {
        assert_eq!(
            installed_exe_call_bytes(site, GET_SLOT_ZERO_FUNCTION_RVA),
            bytes
        );
    }
}

#[test]
fn the_talk_window_restore_site_matches_the_installed_exe_bytes() {
    assert_eq!(
        installed_exe_call_bytes(
            TALK_WINDOW_CLOSE_SLOT_ZERO_RESTORE_CALL_SITE_RVA,
            SET_SLOT_ZERO_FUNCTION_RVA
        ),
        [0xE8, 0x52, 0x1C, 0xED, 0xFF]
    );
}

#[test]
fn a_call_to_a_stub_out_of_reach_is_refused() {
    assert!(near_call_bytes(IMAGE_BASE, IMAGE_BASE + 0x8000_0010).is_none());
    assert!(near_call_bytes(IMAGE_BASE, IMAGE_BASE + 0x7000_0000).is_some());
}

#[test]
fn the_talk_open_save_site_matches_the_installed_exe_bytes() {
    assert_eq!(
        installed_exe_call_bytes(
            TALK_WINDOW_OPEN_SLOT_ZERO_SAVE_CALL_SITE_RVA,
            GET_SLOT_ZERO_FUNCTION_RVA
        ),
        [0xE8, 0x02, 0x00, 0xED, 0xFF]
    );
}

#[test]
fn a_talk_opened_under_a_menu_restores_without_only_the_bits_the_menu_added() {
    let plain_added = menu_added_bits(0, 0x19F);
    assert_eq!(plain_added, 0x19F);
    assert_eq!(talk_window_restored_slot_zero(0x19F, plain_added, false), 0);
    let event_added = menu_added_bits(0x481, 0x481 | 0x19F);
    assert_eq!(event_added, 0x11E);
    assert_eq!(
        talk_window_restored_slot_zero(0x481 | 0x19F, event_added, false),
        0x481
    );
}

#[test]
fn a_talk_restore_is_untouched_while_the_menu_is_open_or_when_it_was_not_opened_under_one() {
    assert_eq!(talk_window_restored_slot_zero(0x19F, 0x19F, true), 0x19F);
    assert_eq!(talk_window_restored_slot_zero(0x481, 0, false), 0x481);
}
