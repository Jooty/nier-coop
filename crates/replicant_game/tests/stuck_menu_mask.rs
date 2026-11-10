use replicant_game::coop_pause::{
    LEFTOVER_PAUSE_MENU_MASK, LEFTOVER_PAUSE_MENU_MASK_GRACE_MS, MaskView, StuckMenuMask,
    StuckOtherSlot,
};

fn closed_menu_with_menu_bits_left() -> MaskView {
    MaskView {
        slots: [LEFTOVER_PAUSE_MENU_MASK, 0, 0, 0],
        extra_ui_scene_open: false,
        ui_dialog_state: 0,
        event_scene: false,
        talk_window_open: false,
        in_play: true,
    }
}

#[test]
fn menu_bits_left_after_the_menu_closed_are_cleared_after_the_grace_time() {
    let mut watch = StuckMenuMask::new();
    let view = closed_menu_with_menu_bits_left();
    assert_eq!(watch.observe(view, 1_000), None);
    assert_eq!(
        watch.observe(view, 1_000 + LEFTOVER_PAUSE_MENU_MASK_GRACE_MS - 1),
        None
    );
    assert_eq!(
        watch.observe(view, 1_000 + LEFTOVER_PAUSE_MENU_MASK_GRACE_MS),
        Some(LEFTOVER_PAUSE_MENU_MASK_GRACE_MS)
    );
}

#[test]
fn an_open_scene_dialog_event_scene_or_loading_screen_is_never_cleared() {
    let mut watch = StuckMenuMask::new();
    for view in [
        MaskView {
            extra_ui_scene_open: true,
            ..closed_menu_with_menu_bits_left()
        },
        MaskView {
            ui_dialog_state: 4,
            ..closed_menu_with_menu_bits_left()
        },
        MaskView {
            event_scene: true,
            ..closed_menu_with_menu_bits_left()
        },
        MaskView {
            in_play: false,
            ..closed_menu_with_menu_bits_left()
        },
    ] {
        assert_eq!(watch.observe(view, 0), None);
        assert_eq!(watch.observe(view, 60_000), None);
    }
}

#[test]
fn other_masks_and_other_slots_are_left_alone() {
    let mut watch = StuckMenuMask::new();
    let scripted_pause = MaskView {
        slots: [0x10, 0, 0, 0],
        ..closed_menu_with_menu_bits_left()
    };
    let talk_open = MaskView {
        slots: [LEFTOVER_PAUSE_MENU_MASK, 0xA0, 0, 0],
        talk_window_open: true,
        ..closed_menu_with_menu_bits_left()
    };
    for view in [scripted_pause, talk_open] {
        assert_eq!(watch.observe(view, 0), None);
        assert_eq!(watch.observe(view, 60_000), None);
    }
}

#[test]
fn a_gap_in_the_leftover_restarts_the_grace_time() {
    let mut watch = StuckMenuMask::new();
    let view = closed_menu_with_menu_bits_left();
    let free = MaskView {
        slots: [0; 4],
        ..view
    };
    assert_eq!(watch.observe(view, 0), None);
    assert_eq!(watch.observe(free, 1_500), None);
    assert_eq!(watch.observe(view, 2_500), None);
    assert_eq!(watch.observe(view, 3_000), None);
}

fn slot_one_held_after_a_talk() -> MaskView {
    MaskView {
        slots: [0, 0xA0, 0, 0],
        ..closed_menu_with_menu_bits_left()
    }
}

#[test]
fn slot_one_left_at_a_0xa0_during_a_talk_is_left_alone() {
    let mut watch = StuckMenuMask::new();
    let view = MaskView {
        talk_window_open: true,
        ..slot_one_held_after_a_talk()
    };
    assert_eq!(watch.observe_other_slots(view, 0), None);
    assert_eq!(watch.observe_other_slots(view, 60_000), None);
}

#[test]
fn slot_one_left_at_a_0xa0_with_nothing_open_is_cleared_after_two_seconds() {
    let mut watch = StuckMenuMask::new();
    let view = slot_one_held_after_a_talk();
    assert_eq!(watch.observe_other_slots(view, 5_000), None);
    assert_eq!(
        watch.observe_other_slots(view, 5_000 + LEFTOVER_PAUSE_MENU_MASK_GRACE_MS - 1),
        None
    );
    assert_eq!(
        watch.observe_other_slots(view, 5_000 + LEFTOVER_PAUSE_MENU_MASK_GRACE_MS),
        Some(StuckOtherSlot {
            slot: 1,
            mask: 0xA0,
            stuck_ms: LEFTOVER_PAUSE_MENU_MASK_GRACE_MS
        })
    );
}

#[test]
fn a_menu_scene_or_loading_screen_keeps_other_slots_untouched() {
    let mut watch = StuckMenuMask::new();
    for view in [
        MaskView {
            extra_ui_scene_open: true,
            ..slot_one_held_after_a_talk()
        },
        MaskView {
            event_scene: true,
            ..slot_one_held_after_a_talk()
        },
        MaskView {
            in_play: false,
            ..slot_one_held_after_a_talk()
        },
    ] {
        assert_eq!(watch.observe_other_slots(view, 0), None);
        assert_eq!(watch.observe_other_slots(view, 60_000), None);
    }
}

#[test]
fn the_slot_zero_leftover_still_clears_and_ignores_the_other_slot_rule() {
    let mut watch = StuckMenuMask::new();
    let view = closed_menu_with_menu_bits_left();
    assert_eq!(watch.observe_other_slots(view, 0), None);
    assert_eq!(watch.observe_other_slots(view, 60_000), None);
    assert_eq!(watch.observe(view, 0), None);
    assert_eq!(
        watch.observe(view, LEFTOVER_PAUSE_MENU_MASK_GRACE_MS),
        Some(LEFTOVER_PAUSE_MENU_MASK_GRACE_MS)
    );
}
