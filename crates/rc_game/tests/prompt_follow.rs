use rc_game::prompt_follow::{
    DIALOG_WAIT_PARKED_WHILE_POLLED_WITHIN_MS, HOST_DISMISSAL_FOLLOWED_OUTSIDE_A_FIGHT_FOR_MS,
    MAX_PROMPT_CLOSE_TRIES_PER_PAGE, PROMPT_BEHIND_STREAM_GRACE_MS, PROMPT_CLOSE_RETRY_AFTER_MS,
    PROMPT_PAGE_SHOWN_DIALOG_STATE, PromptCatchUpClose, PromptClose, PromptDismissal, PromptFollow,
    PromptPageId, PromptStep, PromptView, ScriptDialogNative, dialog_wait_parked,
    format_prompt_catch_up_close, format_prompt_follow_close,
};

const OPENING: i32 = 0x2;
const CLOSING: i32 = 0x4;
const SHOWN: i32 = PROMPT_PAGE_SHOWN_DIALOG_STATE;
const KIND: i32 = 0x9C5;
const TUTORIAL: i32 = 2001;
const OTHER_TUTORIAL: i32 = 2002;
const DIALOG_ID: i32 = 0x55;

fn view(stream_active: bool, dialog_state: i32, now_ms: u64) -> PromptView {
    PromptView {
        stream_active,
        dialog_state,
        pause_menu_open: false,
        now_ms,
        ..PromptView::default()
    }
}

fn page(id: i32, page: u32) -> PromptPageId {
    PromptPageId {
        native: None,
        kind: KIND,
        id,
        page,
    }
}

fn host_dismissed(follow: &mut PromptFollow, page: PromptPageId, times: u32) {
    follow.receive_host_dismissal(PromptDismissal {
        page,
        times,
        in_fight: true,
    });
}

fn show_page(follow: &mut PromptFollow, now_ms: u64) -> PromptStep {
    follow.tick(view(true, OPENING, now_ms));
    follow.tick(view(true, SHOWN, now_ms))
}

fn dismiss_page(follow: &mut PromptFollow, now_ms: u64) -> PromptStep {
    follow.tick(view(true, CLOSING, now_ms))
}

#[test]
fn a_dismissed_page_is_counted_with_its_identity() {
    let mut follow = PromptFollow::new();
    follow.note_open(KIND, TUTORIAL);
    show_page(&mut follow, 0);
    let step = dismiss_page(&mut follow, 1);
    assert_eq!(
        step.dismissed,
        Some(PromptDismissal {
            page: page(TUTORIAL, 1),
            times: 1,
            in_fight: true,
        })
    );
    follow.tick(view(true, 0, 2));
    follow.note_open(KIND, TUTORIAL);
    show_page(&mut follow, 3);
    assert_eq!(dismiss_page(&mut follow, 4).dismissed.unwrap().times, 2);
}

#[test]
fn the_same_identity_pairs_and_closes_one_page_per_host_dismissal() {
    let mut follow = PromptFollow::new();
    host_dismissed(&mut follow, page(TUTORIAL, 1), 1);
    follow.note_open(KIND, TUTORIAL);
    let step = show_page(&mut follow, 0);
    assert_eq!(step.close.unwrap().page, page(TUTORIAL, 1));
    assert_eq!(follow.tick(view(true, SHOWN, 1)).close, None);
    dismiss_page(&mut follow, 2);
    follow.tick(view(true, 0, 3));
    follow.note_open(KIND, TUTORIAL);
    assert_eq!(show_page(&mut follow, 4).close, None);
}

#[test]
fn a_two_page_prompt_closes_page_by_page() {
    let mut follow = PromptFollow::new();
    host_dismissed(&mut follow, page(TUTORIAL, 1), 1);
    host_dismissed(&mut follow, page(TUTORIAL, 2), 1);
    follow.note_open(KIND, TUTORIAL);
    assert_eq!(
        show_page(&mut follow, 0).close.unwrap().page,
        page(TUTORIAL, 1)
    );
    dismiss_page(&mut follow, 1);
    follow.tick(view(true, 0x1, 2));
    follow.tick(view(true, OPENING, 3));
    let second = follow.tick(view(true, SHOWN, 4));
    assert_eq!(second.close.unwrap().page, page(TUTORIAL, 2));
    dismiss_page(&mut follow, 5);
    follow.tick(view(true, 0, 6));
    assert_eq!(follow.dismissed_times(page(TUTORIAL, 2)), 1);
}

#[test]
fn a_host_dismissal_the_guest_never_shows_closes_nothing_not_even_a_later_identity() {
    let mut follow = PromptFollow::new();
    host_dismissed(&mut follow, page(TUTORIAL, 1), 1);
    follow.note_open(KIND, OTHER_TUTORIAL);
    assert_eq!(show_page(&mut follow, 0).close, None);
    dismiss_page(&mut follow, 1);
    follow.tick(view(true, 0, 2));
    follow.note_open(KIND, OTHER_TUTORIAL);
    assert_eq!(show_page(&mut follow, 3).close, None);
}

#[test]
fn a_dialog_with_no_recorded_identity_is_never_closed() {
    let mut follow = PromptFollow::new();
    host_dismissed(&mut follow, page(TUTORIAL, 1), 5);
    assert_eq!(show_page(&mut follow, 0).close, None);
    assert_eq!(dismiss_page(&mut follow, 1).dismissed, None);
}

#[test]
fn an_identity_does_not_outlive_its_dialog() {
    let mut follow = PromptFollow::new();
    follow.note_open(KIND, TUTORIAL);
    show_page(&mut follow, 0);
    dismiss_page(&mut follow, 1);
    follow.tick(view(true, 0, 2));
    host_dismissed(&mut follow, page(TUTORIAL, 1), 9);
    assert_eq!(show_page(&mut follow, 3).close, None);
}

#[test]
fn a_prompt_outside_a_fight_stream_is_untouched() {
    let mut follow = PromptFollow::new();
    host_dismissed(&mut follow, page(TUTORIAL, 1), 3);
    follow.note_open(KIND, TUTORIAL);
    follow.tick(view(false, OPENING, 0));
    assert_eq!(follow.tick(view(false, SHOWN, 1)), PromptStep::default());
    assert_eq!(follow.tick(view(false, CLOSING, 2)), PromptStep::default());
    assert_eq!(follow.dismissed_times(page(TUTORIAL, 1)), 0);
}

#[test]
fn nothing_closes_while_the_pause_menu_is_open() {
    let mut follow = PromptFollow::new();
    host_dismissed(&mut follow, page(TUTORIAL, 1), 1);
    follow.note_open(KIND, TUTORIAL);
    let paused = PromptView {
        pause_menu_open: true,
        ..view(true, SHOWN, 0)
    };
    assert_eq!(follow.tick(paused).close, None);
}

#[test]
fn nothing_closes_unless_the_page_is_in_the_shown_state() {
    let mut follow = PromptFollow::new();
    host_dismissed(&mut follow, page(TUTORIAL, 1), 1);
    follow.note_open(KIND, TUTORIAL);
    assert_eq!(follow.tick(view(true, OPENING, 0)).close, None);
    assert_eq!(follow.tick(view(true, CLOSING, 1)).close, None);
    assert_eq!(follow.tick(view(true, 0x1, 2)).close, None);
}

#[test]
fn a_close_that_does_not_work_is_tried_three_times_then_given_up_once() {
    let mut follow = PromptFollow::new();
    host_dismissed(&mut follow, page(TUTORIAL, 1), 1);
    follow.note_open(KIND, TUTORIAL);
    let mut now = 0;
    let mut tries = Vec::new();
    for _ in 0..6 {
        let step = follow.tick(view(true, SHOWN, now));
        if let Some(close) = step.close {
            tries.push(close.try_number);
        }
        if let Some(gave_up) = step.gave_up_on {
            assert_eq!(gave_up, page(TUTORIAL, 1));
            tries.push(100);
        }
        now += PROMPT_CLOSE_RETRY_AFTER_MS;
    }
    assert_eq!(MAX_PROMPT_CLOSE_TRIES_PER_PAGE, 3);
    assert_eq!(tries, vec![1, 2, 3, 100]);
}

#[test]
fn a_close_waits_before_trying_again() {
    let mut follow = PromptFollow::new();
    host_dismissed(&mut follow, page(TUTORIAL, 1), 1);
    follow.note_open(KIND, TUTORIAL);
    assert!(follow.tick(view(true, SHOWN, 0)).close.is_some());
    let early = view(true, SHOWN, PROMPT_CLOSE_RETRY_AFTER_MS - 1);
    assert_eq!(follow.tick(early).close, None);
}

#[test]
fn counts_reset_when_the_stream_closes_and_opens() {
    let mut follow = PromptFollow::new();
    follow.note_open(KIND, TUTORIAL);
    show_page(&mut follow, 0);
    dismiss_page(&mut follow, 1);
    host_dismissed(&mut follow, page(TUTORIAL, 1), 4);
    follow.tick(view(false, 0, 2));
    assert_eq!(follow.dismissed_times(page(TUTORIAL, 1)), 0);
    assert_eq!(follow.host_dismissed_times(page(TUTORIAL, 1)), 0);
}

fn dialog_page(native: ScriptDialogNative, page: u32) -> PromptPageId {
    PromptPageId {
        native: Some(native),
        kind: DIALOG_ID,
        id: 0,
        page,
    }
}

fn outside_view(dialog_state: i32, now_ms: u64, partner_in_same_phase: bool) -> PromptView {
    PromptView {
        partner_in_same_phase,
        phase: Some(7),
        ..view(false, dialog_state, now_ms)
    }
}

fn catch_up_view(dialog_state: i32, now_ms: u64, entry_behind_ms: Option<u64>) -> PromptView {
    PromptView {
        parked_in_dialog_wait: true,
        entry_behind_ms,
        ..view(true, dialog_state, now_ms)
    }
}

fn host_dismissed_outside(
    follow: &mut PromptFollow,
    page: PromptPageId,
    times: u32,
    received_ms: u64,
) {
    follow.receive_host_dismissal_at(
        PromptDismissal {
            page,
            times,
            in_fight: false,
        },
        received_ms,
    );
}

#[test]
fn a_2d_info_dialog_pairs_by_its_native_and_dialog_id_in_a_fight() {
    let mut follow = PromptFollow::new();
    host_dismissed(
        &mut follow,
        dialog_page(ScriptDialogNative::Start2DInfo, 1),
        1,
    );
    follow.note_dialog_open(Some(ScriptDialogNative::Start2DInfo), DIALOG_ID, 0);
    let step = show_page(&mut follow, 0);
    assert_eq!(
        step.close,
        Some(PromptClose {
            page: dialog_page(ScriptDialogNative::Start2DInfo, 1),
            try_number: 1
        })
    );
    assert_eq!(step.catch_up_close, None);
}

#[test]
fn the_same_dialog_id_opened_by_another_native_does_not_pair() {
    let mut follow = PromptFollow::new();
    host_dismissed(
        &mut follow,
        dialog_page(ScriptDialogNative::StartInfoItem, 1),
        1,
    );
    follow.note_dialog_open(Some(ScriptDialogNative::Start2DInfo), DIALOG_ID, 0);
    assert_eq!(show_page(&mut follow, 0).close, None);
}

#[test]
fn every_dialog_native_pairs_inside_a_fight() {
    for native in ScriptDialogNative::ALL {
        let mut follow = PromptFollow::new();
        host_dismissed(&mut follow, dialog_page(native, 1), 1);
        follow.note_dialog_open(Some(native), DIALOG_ID, 0);
        assert!(show_page(&mut follow, 0).close.is_some(), "{native:?}");
    }
}

#[test]
fn outside_a_fight_a_2d_info_or_tutorial_follows_the_host_in_the_same_phase() {
    for native in [
        ScriptDialogNative::Start2DInfo,
        ScriptDialogNative::StartInfoTutorial,
    ] {
        let mut follow = PromptFollow::new();
        follow.tick(outside_view(0, 0, true));
        host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 0);
        follow.note_dialog_open(Some(native), DIALOG_ID, 0);
        follow.tick(outside_view(OPENING, 1, true));
        let step = follow.tick(outside_view(SHOWN, 2, true));
        assert_eq!(
            step.close.unwrap().page,
            dialog_page(native, 1),
            "{native:?}"
        );
    }
}

#[test]
fn outside_a_fight_item_money_quest_and_word_windows_stay_each_players_own() {
    for native in [
        ScriptDialogNative::StartInfoItem,
        ScriptDialogNative::StartInfoMoney,
        ScriptDialogNative::StartInfoQuest,
        ScriptDialogNative::StartInfoWord,
    ] {
        assert!(!native.follows_outside_a_fight());
        let mut follow = PromptFollow::new();
        follow.tick(outside_view(0, 0, true));
        host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 0);
        follow.note_dialog_open(Some(native), DIALOG_ID, 0);
        follow.tick(outside_view(OPENING, 1, true));
        assert_eq!(follow.tick(outside_view(SHOWN, 2, true)).close, None);
        assert_eq!(follow.tick(outside_view(CLOSING, 3, true)).dismissed, None);
    }
}

#[test]
fn outside_a_fight_nothing_follows_while_the_partner_is_in_another_phase() {
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, false));
    host_dismissed_outside(
        &mut follow,
        dialog_page(ScriptDialogNative::Start2DInfo, 1),
        1,
        0,
    );
    follow.note_dialog_open(Some(ScriptDialogNative::Start2DInfo), DIALOG_ID, 0);
    follow.tick(outside_view(OPENING, 1, false));
    assert_eq!(follow.tick(outside_view(SHOWN, 2, false)).close, None);
    assert_eq!(follow.tick(outside_view(CLOSING, 3, false)).dismissed, None);
}

#[test]
fn outside_a_fight_a_tutorial_prompt_with_no_known_wait_is_untouched() {
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    host_dismissed_outside(&mut follow, page(TUTORIAL, 1), 1, 0);
    follow.note_open(KIND, TUTORIAL);
    follow.tick(outside_view(OPENING, 1, true));
    assert_eq!(follow.tick(outside_view(SHOWN, 2, true)).close, None);
}

#[test]
fn outside_a_fight_a_host_count_taken_in_the_fight_is_not_followed() {
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    host_dismissed(
        &mut follow,
        dialog_page(ScriptDialogNative::Start2DInfo, 1),
        1,
    );
    follow.note_dialog_open(Some(ScriptDialogNative::Start2DInfo), DIALOG_ID, 0);
    follow.tick(outside_view(OPENING, 1, true));
    assert_eq!(follow.tick(outside_view(SHOWN, 2, true)).close, None);
}

#[test]
fn outside_a_fight_an_old_host_count_is_not_followed() {
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    host_dismissed_outside(
        &mut follow,
        dialog_page(ScriptDialogNative::Start2DInfo, 1),
        1,
        0,
    );
    follow.note_dialog_open(Some(ScriptDialogNative::Start2DInfo), DIALOG_ID, 0);
    let late = HOST_DISMISSAL_FOLLOWED_OUTSIDE_A_FIGHT_FOR_MS + 1;
    follow.tick(outside_view(OPENING, late, true));
    assert_eq!(follow.tick(outside_view(SHOWN, late, true)).close, None);
}

#[test]
fn the_host_counts_a_2d_info_outside_a_fight_only_with_the_partner_in_the_same_phase() {
    let mut follow = PromptFollow::new();
    follow.note_dialog_open(Some(ScriptDialogNative::Start2DInfo), DIALOG_ID, 0);
    follow.tick(outside_view(OPENING, 0, true));
    follow.tick(outside_view(SHOWN, 1, true));
    assert_eq!(
        follow.tick(outside_view(CLOSING, 2, true)).dismissed,
        Some(PromptDismissal {
            page: dialog_page(ScriptDialogNative::Start2DInfo, 1),
            times: 1,
            in_fight: false,
        })
    );
    follow.tick(outside_view(0, 3, false));
    follow.note_dialog_open(Some(ScriptDialogNative::Start2DInfo), DIALOG_ID, 0);
    follow.tick(outside_view(OPENING, 4, false));
    follow.tick(outside_view(SHOWN, 5, false));
    assert_eq!(follow.tick(outside_view(CLOSING, 6, false)).dismissed, None);
}

#[test]
fn counts_start_over_when_the_phase_changes() {
    let mut follow = PromptFollow::new();
    follow.note_dialog_open(Some(ScriptDialogNative::Start2DInfo), DIALOG_ID, 0);
    follow.tick(outside_view(OPENING, 0, true));
    follow.tick(outside_view(SHOWN, 1, true));
    follow.tick(outside_view(CLOSING, 2, true));
    host_dismissed_outside(
        &mut follow,
        dialog_page(ScriptDialogNative::Start2DInfo, 1),
        3,
        2,
    );
    let next_phase = PromptView {
        phase: Some(8),
        ..outside_view(0, 3, true)
    };
    follow.tick(next_phase);
    let page = dialog_page(ScriptDialogNative::Start2DInfo, 1);
    assert_eq!(follow.dismissed_times(page), 0);
    assert_eq!(follow.host_dismissed_times(page), 0);
}

#[test]
fn a_parked_dialog_wait_closes_once_a_host_entry_waited_past_the_grace() {
    let mut follow = PromptFollow::new();
    follow.tick(catch_up_view(OPENING, 0, None));
    let at_grace = follow.tick(catch_up_view(SHOWN, 1, Some(PROMPT_BEHIND_STREAM_GRACE_MS)));
    assert_eq!(at_grace.catch_up_close, None);
    let past = follow.tick(catch_up_view(
        SHOWN,
        2,
        Some(PROMPT_BEHIND_STREAM_GRACE_MS + 1),
    ));
    assert_eq!(
        past.catch_up_close,
        Some(PromptCatchUpClose {
            try_number: 1,
            entry_behind_ms: PROMPT_BEHIND_STREAM_GRACE_MS + 1,
        })
    );
    assert_eq!(past.close, None);
}

#[test]
fn the_catch_up_needs_the_slot_parked_in_a_dialog_wait() {
    let mut follow = PromptFollow::new();
    let not_parked = PromptView {
        parked_in_dialog_wait: false,
        ..catch_up_view(SHOWN, 0, Some(60_000))
    };
    assert_eq!(follow.tick(not_parked), PromptStep::default());
}

#[test]
fn the_catch_up_never_closes_outside_a_fight_under_the_pause_menu_or_off_the_shown_page() {
    let mut follow = PromptFollow::new();
    let outside = PromptView {
        stream_active: false,
        ..catch_up_view(SHOWN, 0, Some(60_000))
    };
    assert_eq!(follow.tick(outside).catch_up_close, None);
    let paused = PromptView {
        pause_menu_open: true,
        ..catch_up_view(SHOWN, 1, Some(60_000))
    };
    assert_eq!(follow.tick(paused).catch_up_close, None);
    for state in [0, 0x1, OPENING, CLOSING] {
        assert_eq!(
            follow
                .tick(catch_up_view(state, 2, Some(60_000)))
                .catch_up_close,
            None
        );
    }
}

#[test]
fn the_catch_up_tries_three_times_then_gives_up_once() {
    let mut follow = PromptFollow::new();
    let mut now = 0;
    let mut tries = Vec::new();
    for _ in 0..6 {
        let step = follow.tick(catch_up_view(SHOWN, now, Some(1_000)));
        if let Some(close) = step.catch_up_close {
            tries.push(close.try_number);
        }
        if step.catch_up_gave_up {
            tries.push(100);
        }
        now += PROMPT_CLOSE_RETRY_AFTER_MS;
    }
    assert_eq!(tries, vec![1, 2, 3, 100]);
}

#[test]
fn a_paired_close_comes_before_the_catch_up() {
    let mut follow = PromptFollow::new();
    host_dismissed(
        &mut follow,
        dialog_page(ScriptDialogNative::Start2DInfo, 1),
        1,
    );
    follow.note_dialog_open(Some(ScriptDialogNative::Start2DInfo), DIALOG_ID, 0);
    follow.tick(catch_up_view(OPENING, 0, Some(1_000)));
    let step = follow.tick(catch_up_view(SHOWN, 1, Some(1_000)));
    assert!(step.close.is_some());
    assert_eq!(step.catch_up_close, None);
}

#[test]
fn a_dialog_wait_is_parked_only_while_its_poll_still_waited_lately() {
    assert!(!dialog_wait_parked(None, 0));
    assert!(dialog_wait_parked(Some(100), 100));
    assert!(dialog_wait_parked(
        Some(100),
        100 + DIALOG_WAIT_PARKED_WHILE_POLLED_WITHIN_MS
    ));
    assert!(!dialog_wait_parked(
        Some(100),
        101 + DIALOG_WAIT_PARKED_WHILE_POLLED_WITHIN_MS
    ));
}

#[test]
fn every_dialog_native_has_its_own_wire_code_and_poll() {
    let mut codes = Vec::new();
    let mut polls = Vec::new();
    for native in ScriptDialogNative::ALL {
        assert_eq!(
            ScriptDialogNative::from_wire_code(native.wire_code()),
            Some(native)
        );
        codes.push(native.wire_code());
        polls.push(native.poll_rva());
    }
    codes.dedup();
    polls.sort_unstable();
    polls.dedup();
    assert_eq!(codes, vec![1, 2, 3, 4, 5, 6]);
    assert_eq!(polls.len(), 6);
    assert_eq!(ScriptDialogNative::from_wire_code(0), None);
    assert_eq!(ScriptDialogNative::from_wire_code(7), None);
    assert!(!ScriptDialogNative::StartInfoTutorial.opens_through_its_poll());
}

#[test]
fn the_close_lines_name_the_page_and_the_reason() {
    let close = PromptClose {
        page: dialog_page(ScriptDialogNative::Start2DInfo, 1),
        try_number: 1,
    };
    assert_eq!(
        format_prompt_follow_close(close),
        "prompt follow: closing your _Start2DInfo 85/0 page 1, the host dismissed it"
    );
    let catch_up = PromptCatchUpClose {
        try_number: 2,
        entry_behind_ms: 640,
    };
    assert_eq!(
        format_prompt_catch_up_close(
            catch_up,
            Some(ScriptDialogNative::StartInfoItem),
            "EID_0200_a0060_End"
        ),
        "prompt follow: closing your page, the host moved on (_StartInfoItem in EID_0200_a0060_End, try 2, a host entry waited 640 ms behind it)"
    );
}
