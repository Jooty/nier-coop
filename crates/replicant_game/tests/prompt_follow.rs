use replicant_game::prompt_follow::{
    DIALOG_WAIT_PARKED_WHILE_POLLED_WITHIN_MS, HOST_DISMISSAL_FOLLOWED_OUTSIDE_A_FIGHT_FOR_MS,
    MAX_PROMPT_CLOSE_TRIES_PER_PAGE,
    PARTNER_CLOSE_COUNTS_IF_SEEN_BEFORE_THIS_COPY_OPENED_WITHIN_MS, PROMPT_BEHIND_STREAM_GRACE_MS,
    PROMPT_CLOSE_RETRY_AFTER_MS, PROMPT_PAGE_SHOWN_DIALOG_STATE, PromptCatchUpClose, PromptClose,
    PromptDismissal, PromptFollow, PromptPageId, PromptStep, PromptView,
    REWARD_PROMPT_KEPT_ON_SCREEN_FOR_AT_LEAST_MS, ScriptDialogNative, dialog_wait_parked,
    format_prompt_catch_up_close, format_prompt_follow_close, format_prompt_opened,
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
fn outside_a_fight_money_and_word_windows_stay_each_players_own() {
    for native in [
        ScriptDialogNative::StartInfoMoney,
        ScriptDialogNative::StartInfoWord,
    ] {
        assert!(!native.follows_outside_a_fight());
        let mut follow = PromptFollow::new();
        follow.tick(outside_view(0, 0, true));
        host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 0);
        follow.note_dialog_open(Some(native), DIALOG_ID, 0);
        follow.tick(outside_view(OPENING, 1, true));
        assert_eq!(follow.tick(outside_view(SHOWN, 2, true)).close, None);
        follow.tick(outside_view(CLOSING, 3, true));
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
        "prompt follow: closing your _Start2DInfo 85/0 page 1, your partner dismissed it"
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

const HANDLER_SINCE_MS: u64 = 100;

fn open_host_led(follow: &mut PromptFollow, native: ScriptDialogNative, now_ms: u64) {
    follow.note_dialog_open_in(Some(native), DIALOG_ID, 0, Some(HANDLER_SINCE_MS));
    follow.tick(outside_view(OPENING, now_ms, true));
}

fn show_host_led(follow: &mut PromptFollow, native: ScriptDialogNative, now_ms: u64) -> PromptStep {
    open_host_led(follow, native, now_ms);
    follow.tick(outside_view(SHOWN, now_ms + 1, true))
}

fn end_dialog(follow: &mut PromptFollow, now_ms: u64) {
    follow.tick(outside_view(CLOSING, now_ms, true));
    follow.tick(outside_view(0, now_ms + 1, true));
}

#[test]
fn a_host_led_item_popup_outside_a_fight_closes_when_the_host_dismisses_it_afterwards() {
    let native = ScriptDialogNative::StartInfoItem;
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    let step = show_host_led(&mut follow, native, 200);
    assert_eq!(step.close, None);
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 5_000);
    let step = follow.tick(outside_view(SHOWN, 5_001, true));
    assert_eq!(step.close.unwrap().page, dialog_page(native, 1));
}

#[test]
fn a_host_led_popup_has_no_thirty_second_limit() {
    let native = ScriptDialogNative::StartInfoItem;
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    show_host_led(&mut follow, native, 200);
    let late = HOST_DISMISSAL_FOLLOWED_OUTSIDE_A_FIGHT_FOR_MS * 4;
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, late);
    let step = follow.tick(outside_view(SHOWN, late + 1, true));
    assert!(step.close.is_some());
}

#[test]
fn a_host_dismissal_from_before_the_popup_opened_but_after_the_announcement_closes_it() {
    let native = ScriptDialogNative::StartInfoItem;
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, HANDLER_SINCE_MS + 5);
    assert_eq!(show_host_led(&mut follow, native, 400).close, None);
    let step = follow.tick(outside_view(SHOWN, 2_000, true));
    assert_eq!(step.close.unwrap().page, dialog_page(native, 1));
}

#[test]
fn a_host_dismissal_from_before_the_announcement_does_not_close_it() {
    let native = ScriptDialogNative::StartInfoItem;
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, HANDLER_SINCE_MS - 1);
    let step = show_host_led(&mut follow, native, 400);
    assert_eq!(step.close, None);
    assert_eq!(follow.tick(outside_view(SHOWN, 500, true)).close, None);
}

#[test]
fn a_host_led_popup_needs_the_partner_in_the_same_phase() {
    let native = ScriptDialogNative::StartInfoItem;
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, false));
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 300);
    follow.note_dialog_open_in(Some(native), DIALOG_ID, 0, Some(HANDLER_SINCE_MS));
    follow.tick(outside_view(OPENING, 301, false));
    assert_eq!(follow.tick(outside_view(SHOWN, 302, false)).close, None);
}

#[test]
fn money_then_quest_in_one_host_led_handler_each_close_on_their_own_host_dismissal() {
    let money = ScriptDialogNative::StartInfoMoney;
    let quest = ScriptDialogNative::StartInfoQuest;
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    show_host_led(&mut follow, money, 200);
    host_dismissed_outside(&mut follow, dialog_page(money, 1), 1, 300);
    let step = follow.tick(outside_view(SHOWN, 301, true));
    assert_eq!(step.close.unwrap().page, dialog_page(money, 1));
    end_dialog(&mut follow, 302);
    let step = show_host_led(&mut follow, quest, 400);
    assert_eq!(step.close, None);
    host_dismissed_outside(&mut follow, dialog_page(quest, 1), 1, 500);
    let step = follow.tick(outside_view(SHOWN, 2_000, true));
    assert_eq!(step.close.unwrap().page, dialog_page(quest, 1));
}

#[test]
fn a_two_page_host_led_popup_closes_page_by_page() {
    let native = ScriptDialogNative::StartInfoItem;
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    show_host_led(&mut follow, native, 200);
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 300);
    let step = follow.tick(outside_view(SHOWN, 2_000, true));
    assert_eq!(step.close.unwrap().page, dialog_page(native, 1));
    follow.tick(outside_view(CLOSING, 2_002, true));
    follow.tick(outside_view(OPENING, 2_003, true));
    let step = follow.tick(outside_view(SHOWN, 2_004, true));
    assert_eq!(step.close, None);
    host_dismissed_outside(&mut follow, dialog_page(native, 2), 1, 2_100);
    let step = follow.tick(outside_view(SHOWN, 4_000, true));
    assert_eq!(step.close.unwrap().page, dialog_page(native, 2));
}

#[test]
fn a_host_count_from_the_hosts_own_earlier_popup_does_not_close_a_repeat_in_the_same_handler() {
    let native = ScriptDialogNative::StartInfoItem;
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 10);
    show_host_led(&mut follow, native, 200);
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 2, 300);
    let step = follow.tick(outside_view(SHOWN, 2_000, true));
    assert!(step.close.is_some());
    end_dialog(&mut follow, 2_002);
    let step = show_host_led(&mut follow, native, 2_400);
    assert_eq!(step.close, None);
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 3, 2_500);
    let step = follow.tick(outside_view(SHOWN, 4_000, true));
    assert!(step.close.is_some());
}

#[test]
fn the_host_counts_every_dialog_native_outside_a_fight_only_with_the_partner_in_the_same_phase() {
    for native in ScriptDialogNative::ALL {
        let mut follow = PromptFollow::new();
        follow.note_dialog_open(Some(native), DIALOG_ID, 0);
        follow.tick(outside_view(OPENING, 0, true));
        follow.tick(outside_view(SHOWN, 1, true));
        assert_eq!(
            follow.tick(outside_view(CLOSING, 2, true)).dismissed,
            Some(PromptDismissal {
                page: dialog_page(native, 1),
                times: 1,
                in_fight: false,
            }),
            "{native:?}"
        );
        follow.tick(outside_view(0, 3, false));
        follow.note_dialog_open(Some(native), DIALOG_ID, 0);
        follow.tick(outside_view(OPENING, 4, false));
        follow.tick(outside_view(SHOWN, 5, false));
        assert_eq!(
            follow.tick(outside_view(CLOSING, 6, false)).dismissed,
            None,
            "{native:?}"
        );
    }
}

#[test]
fn the_opened_line_names_the_host_led_handler() {
    let native = Some(ScriptDialogNative::StartInfoItem);
    assert_eq!(
        format_prompt_opened(native, 85, 0, None),
        "prompt follow: _StartInfoItem 85/0 opened"
    );
    assert_eq!(
        format_prompt_opened(native, 85, 0, Some("SEID_0001_End")),
        "prompt follow: _StartInfoItem 85/0 opened, inside the host-led handler SEID_0001_End"
    );
}

#[test]
fn a_host_dismissal_received_after_the_handler_arrived_but_before_it_was_queued_still_closes() {
    let native = ScriptDialogNative::StartInfoItem;
    let arrived_ms = 100;
    let queued_ms = 900;
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 500);
    assert!(queued_ms > 500);
    follow.note_dialog_open_in(Some(native), DIALOG_ID, 0, Some(arrived_ms));
    follow.tick(outside_view(OPENING, 1_000, true));
    assert_eq!(follow.tick(outside_view(SHOWN, 1_001, true)).close, None);
    let step = follow.tick(outside_view(SHOWN, 3_000, true));
    assert_eq!(step.close.unwrap().page, dialog_page(native, 1));
}

fn open_reward_prompt(follow: &mut PromptFollow, native: ScriptDialogNative, shown_ms: u64) {
    follow.tick(outside_view(0, shown_ms.saturating_sub(2), true));
    follow.note_dialog_open(Some(native), DIALOG_ID, 0);
    follow.tick(outside_view(OPENING, shown_ms.saturating_sub(1), true));
    assert_eq!(follow.tick(outside_view(SHOWN, shown_ms, true)).close, None);
}

const REWARD_PROMPTS: [ScriptDialogNative; 2] = [
    ScriptDialogNative::StartInfoItem,
    ScriptDialogNative::StartInfoQuest,
];

#[test]
fn a_reward_prompt_open_on_both_closes_when_the_partner_closes_theirs() {
    for native in REWARD_PROMPTS {
        let mut follow = PromptFollow::new();
        open_reward_prompt(&mut follow, native, 1_000);
        host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 1_700);
        let step = follow.tick(outside_view(SHOWN, 2_600, true));
        assert_eq!(
            step.close.unwrap().page,
            dialog_page(native, 1),
            "{native:?}"
        );
    }
}

#[test]
fn a_followed_reward_prompt_stays_on_screen_for_at_least_the_floor() {
    let native = ScriptDialogNative::StartInfoItem;
    let mut follow = PromptFollow::new();
    open_reward_prompt(&mut follow, native, 1_000);
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 1_100);
    let before_floor = 1_000 + REWARD_PROMPT_KEPT_ON_SCREEN_FOR_AT_LEAST_MS - 1;
    assert_eq!(
        follow.tick(outside_view(SHOWN, before_floor, true)).close,
        None
    );
    let at_floor = 1_000 + REWARD_PROMPT_KEPT_ON_SCREEN_FOR_AT_LEAST_MS;
    assert!(
        follow
            .tick(outside_view(SHOWN, at_floor, true))
            .close
            .is_some()
    );
}

#[test]
fn a_players_own_press_on_a_reward_prompt_closes_it_at_once_and_is_counted() {
    let native = ScriptDialogNative::StartInfoItem;
    let mut follow = PromptFollow::new();
    open_reward_prompt(&mut follow, native, 1_000);
    let step = follow.tick(outside_view(CLOSING, 1_100, true));
    assert_eq!(
        step.dismissed,
        Some(PromptDismissal {
            page: dialog_page(native, 1),
            times: 1,
            in_fight: false,
        })
    );
}

#[test]
fn a_reward_prompt_open_on_this_copy_only_is_not_closed_by_an_old_partner_count() {
    let native = ScriptDialogNative::StartInfoItem;
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 100);
    let opened_much_later =
        100 + PARTNER_CLOSE_COUNTS_IF_SEEN_BEFORE_THIS_COPY_OPENED_WITHIN_MS + 1;
    open_reward_prompt(&mut follow, native, opened_much_later);
    let step = follow.tick(outside_view(SHOWN, opened_much_later + 5_000, true));
    assert_eq!(step.close, None);
}

#[test]
fn a_partner_close_just_before_this_copy_showed_the_same_prompt_still_closes_it() {
    let native = ScriptDialogNative::StartInfoItem;
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 900);
    open_reward_prompt(&mut follow, native, 1_000);
    let step = follow.tick(outside_view(SHOWN, 2_600, true));
    assert!(step.close.is_some());
}

#[test]
fn a_reward_prompt_already_closed_here_is_not_closed_again_by_the_partners_echo() {
    let native = ScriptDialogNative::StartInfoQuest;
    let mut follow = PromptFollow::new();
    open_reward_prompt(&mut follow, native, 1_000);
    follow.tick(outside_view(CLOSING, 1_100, true));
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 1_200);
    follow.tick(outside_view(0, 1_300, true));
    open_reward_prompt(&mut follow, native, 5_000);
    let step = follow.tick(outside_view(SHOWN, 7_000, true));
    assert_eq!(step.close, None);
}

#[test]
fn a_reward_prompt_follows_the_partner_again_for_a_repeat_of_the_same_prompt() {
    let native = ScriptDialogNative::StartInfoItem;
    let mut follow = PromptFollow::new();
    open_reward_prompt(&mut follow, native, 1_000);
    follow.tick(outside_view(CLOSING, 1_100, true));
    follow.tick(outside_view(0, 1_200, true));
    open_reward_prompt(&mut follow, native, 5_000);
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 2, 5_200);
    let step = follow.tick(outside_view(SHOWN, 6_600, true));
    assert!(step.close.is_some());
}

#[test]
fn a_reward_prompt_needs_the_partner_in_the_same_phase() {
    let native = ScriptDialogNative::StartInfoItem;
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, false));
    host_dismissed_outside(&mut follow, dialog_page(native, 1), 1, 100);
    follow.note_dialog_open(Some(native), DIALOG_ID, 0);
    follow.tick(outside_view(OPENING, 999, false));
    follow.tick(outside_view(SHOWN, 1_000, false));
    assert_eq!(follow.tick(outside_view(SHOWN, 5_000, false)).close, None);
    assert_eq!(
        follow.tick(outside_view(CLOSING, 5_100, false)).dismissed,
        None
    );
}

#[test]
fn inside_a_fight_a_reward_prompt_follows_without_the_floor() {
    for native in REWARD_PROMPTS {
        let mut follow = PromptFollow::new();
        host_dismissed(&mut follow, dialog_page(native, 1), 1);
        follow.note_dialog_open(Some(native), DIALOG_ID, 0);
        assert!(show_page(&mut follow, 0).close.is_some(), "{native:?}");
    }
}

#[test]
fn only_item_and_quest_pop_ups_are_reward_prompts() {
    for native in ScriptDialogNative::ALL {
        let expected = matches!(
            native,
            ScriptDialogNative::StartInfoItem | ScriptDialogNative::StartInfoQuest
        );
        assert_eq!(native.is_reward_prompt(), expected, "{native:?}");
    }
}

#[test]
fn the_skip_vote_prompt_is_never_closed_for_its_player() {
    // The run: the host's answer to prompt 2437 (17:20:40.313) must not answer the guest's copy.
    let vote_page = PromptPageId {
        native: Some(ScriptDialogNative::Start2DInfo),
        kind: 2437,
        id: 0,
        page: 1,
    };
    assert!(vote_page.is_a_skip_vote_page());
    assert!(!dialog_page(ScriptDialogNative::Start2DInfo, 1).is_a_skip_vote_page());
    let mut follow = PromptFollow::new();
    follow.tick(outside_view(0, 0, true));
    host_dismissed_outside(&mut follow, vote_page, 1, 0);
    follow.note_dialog_open(Some(ScriptDialogNative::Start2DInfo), 2437, 0);
    follow.tick(outside_view(OPENING, 1, true));
    for now_ms in (2..20_000).step_by(250) {
        let step = follow.tick(outside_view(SHOWN, now_ms, true));
        assert_eq!(step.close, None);
        assert_eq!(step.unattended_close, None);
    }
}
