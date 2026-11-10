use rc_game::prompt_follow::{
    MAX_PROMPT_CLOSE_TRIES_PER_PAGE, PROMPT_CLOSE_RETRY_AFTER_MS, PROMPT_PAGE_SHOWN_DIALOG_STATE,
    PromptFollow, PromptStep, PromptView,
};

const OPENING: i32 = 0x2;
const CLOSING: i32 = 0x4;

fn view(stream_active: bool, dialog_state: i32, now_ms: u64) -> PromptView {
    PromptView {
        stream_active,
        dialog_state,
        pause_menu_open: false,
        now_ms,
    }
}

fn show_and_dismiss_page(follow: &mut PromptFollow, now_ms: u64) -> PromptStep {
    follow.tick(view(true, OPENING, now_ms));
    follow.tick(view(true, PROMPT_PAGE_SHOWN_DIALOG_STATE, now_ms));
    follow.tick(view(true, CLOSING, now_ms))
}

#[test]
fn a_dismissed_page_is_counted_and_sent() {
    let mut follow = PromptFollow::new();
    assert_eq!(show_and_dismiss_page(&mut follow, 0).send_count, Some(1));
    assert_eq!(show_and_dismiss_page(&mut follow, 10).send_count, Some(2));
    assert_eq!(follow.dismissed(), 2);
}

#[test]
fn a_page_that_goes_straight_to_closed_is_counted_once() {
    let mut follow = PromptFollow::new();
    follow.tick(view(true, PROMPT_PAGE_SHOWN_DIALOG_STATE, 0));
    assert_eq!(follow.tick(view(true, 0, 1)).send_count, Some(1));
    assert_eq!(follow.tick(view(true, 0, 2)).send_count, None);
}

#[test]
fn a_host_ahead_closes_one_page_per_shown_page() {
    let mut follow = PromptFollow::new();
    follow.receive_host_count(2);
    follow.tick(view(true, OPENING, 0));
    let first = follow.tick(view(true, PROMPT_PAGE_SHOWN_DIALOG_STATE, 1));
    let ask = first.close.unwrap();
    assert_eq!((ask.page, ask.host_dismissed, ask.try_number), (1, 2, 1));
    assert_eq!(
        follow
            .tick(view(true, PROMPT_PAGE_SHOWN_DIALOG_STATE, 2))
            .close,
        None
    );
    follow.tick(view(true, CLOSING, 3));
    follow.tick(view(true, OPENING, 4));
    let second = follow.tick(view(true, PROMPT_PAGE_SHOWN_DIALOG_STATE, 5));
    assert_eq!(second.close.unwrap().page, 2);
    follow.tick(view(true, 0, 6));
    follow.tick(view(true, PROMPT_PAGE_SHOWN_DIALOG_STATE, 7));
    assert_eq!(
        follow
            .tick(view(true, PROMPT_PAGE_SHOWN_DIALOG_STATE, 8))
            .close,
        None
    );
}

#[test]
fn equal_counts_close_nothing() {
    let mut follow = PromptFollow::new();
    show_and_dismiss_page(&mut follow, 0);
    follow.receive_host_count(1);
    follow.tick(view(true, OPENING, 1));
    assert_eq!(
        follow
            .tick(view(true, PROMPT_PAGE_SHOWN_DIALOG_STATE, 2))
            .close,
        None
    );
}

#[test]
fn a_host_that_dismissed_nothing_closes_nothing() {
    let mut follow = PromptFollow::new();
    follow.tick(view(true, OPENING, 0));
    assert_eq!(
        follow
            .tick(view(true, PROMPT_PAGE_SHOWN_DIALOG_STATE, 1))
            .close,
        None
    );
}

#[test]
fn a_prompt_outside_a_fight_stream_is_untouched() {
    let mut follow = PromptFollow::new();
    follow.receive_host_count(3);
    for now in 0..5 {
        let step = follow.tick(view(false, PROMPT_PAGE_SHOWN_DIALOG_STATE, now));
        assert_eq!(step, PromptStep::default());
    }
    follow.tick(view(false, CLOSING, 6));
    assert_eq!(follow.dismissed(), 0);
}

#[test]
fn nothing_closes_while_the_pause_menu_is_open() {
    let mut follow = PromptFollow::new();
    follow.receive_host_count(1);
    let paused = PromptView {
        pause_menu_open: true,
        ..view(true, PROMPT_PAGE_SHOWN_DIALOG_STATE, 0)
    };
    assert_eq!(follow.tick(paused).close, None);
}

#[test]
fn nothing_closes_unless_the_page_is_in_the_shown_state() {
    let mut follow = PromptFollow::new();
    follow.receive_host_count(1);
    assert_eq!(follow.tick(view(true, OPENING, 0)).close, None);
    assert_eq!(follow.tick(view(true, CLOSING, 1)).close, None);
    assert_eq!(follow.tick(view(true, 0x1, 2)).close, None);
}

#[test]
fn a_close_that_does_not_work_is_tried_three_times_then_given_up_once() {
    let mut follow = PromptFollow::new();
    follow.receive_host_count(1);
    let mut now = 0;
    let mut tries = Vec::new();
    for _ in 0..6 {
        let step = follow.tick(view(true, PROMPT_PAGE_SHOWN_DIALOG_STATE, now));
        if let Some(close) = step.close {
            tries.push(close.try_number);
        }
        if let Some(page) = step.gave_up_on_page {
            assert_eq!(page, 1);
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
    follow.receive_host_count(1);
    assert!(
        follow
            .tick(view(true, PROMPT_PAGE_SHOWN_DIALOG_STATE, 0))
            .close
            .is_some()
    );
    assert_eq!(
        follow
            .tick(view(
                true,
                PROMPT_PAGE_SHOWN_DIALOG_STATE,
                PROMPT_CLOSE_RETRY_AFTER_MS - 1
            ))
            .close,
        None
    );
}

#[test]
fn counts_reset_when_the_stream_closes_and_opens() {
    let mut follow = PromptFollow::new();
    show_and_dismiss_page(&mut follow, 0);
    follow.receive_host_count(4);
    let closed = follow.tick(view(false, 0, 1));
    assert_eq!(closed.send_count, Some(0));
    assert_eq!((follow.dismissed(), follow.host_dismissed()), (0, 0));
    follow.receive_host_count(1);
    follow.tick(view(true, 0, 2));
    assert_eq!((follow.dismissed(), follow.host_dismissed()), (0, 1));
}
