use rc_game::prompt_follow::{
    MAX_PROMPT_CLOSE_TRIES_PER_PAGE, PROMPT_CLOSE_RETRY_AFTER_MS, PROMPT_PAGE_SHOWN_DIALOG_STATE,
    PromptDismissal, PromptFollow, PromptPageId, PromptStep, PromptView,
};

const OPENING: i32 = 0x2;
const CLOSING: i32 = 0x4;
const SHOWN: i32 = PROMPT_PAGE_SHOWN_DIALOG_STATE;
const KIND: i32 = 0x9C5;
const TUTORIAL: i32 = 2001;
const OTHER_TUTORIAL: i32 = 2002;

fn view(stream_active: bool, dialog_state: i32, now_ms: u64) -> PromptView {
    PromptView {
        stream_active,
        dialog_state,
        pause_menu_open: false,
        now_ms,
    }
}

fn page(id: i32, page: u32) -> PromptPageId {
    PromptPageId {
        kind: KIND,
        id,
        page,
    }
}

fn host_dismissed(follow: &mut PromptFollow, page: PromptPageId, times: u32) {
    follow.receive_host_dismissal(PromptDismissal { page, times });
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
            times: 1
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
