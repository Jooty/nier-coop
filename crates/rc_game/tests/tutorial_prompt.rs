use rc_game::tutorial_prompt::{LocalOpen, PromptView, TutorialPrompts, may_open_now};

const TUTORIAL_ID: u16 = 2004;
const OTHER_TUTORIAL_ID: u16 = 2022;

fn free_view() -> PromptView {
    PromptView {
        in_cutscene: false,
        fade_active: false,
        ui_dialog_state: 0,
        pause_menu_open: false,
    }
}

#[test]
fn a_free_game_may_open_a_prompt() {
    assert!(may_open_now(free_view()));
}

#[test]
fn a_cutscene_a_fade_an_open_dialog_or_the_pause_menu_blocks_it() {
    let blocked = [
        PromptView {
            in_cutscene: true,
            ..free_view()
        },
        PromptView {
            fade_active: true,
            ..free_view()
        },
        PromptView {
            ui_dialog_state: 2,
            ..free_view()
        },
        PromptView {
            pause_menu_open: true,
            ..free_view()
        },
    ];
    for view in blocked {
        assert!(!may_open_now(view), "{view:?}");
    }
}

#[test]
fn a_prompt_queued_during_a_cutscene_opens_after_it() {
    let mut prompts = TutorialPrompts::new();
    assert!(prompts.partner_opened(TUTORIAL_ID));
    assert_eq!(prompts.next_to_open(false), None);
    assert_eq!(prompts.waiting_count(), 1);
    assert_eq!(prompts.next_to_open(true), Some(TUTORIAL_ID));
    assert_eq!(prompts.waiting_count(), 0);
}

#[test]
fn each_id_is_shown_once() {
    let mut prompts = TutorialPrompts::new();
    assert_eq!(prompts.local_opened(TUTORIAL_ID), LocalOpen::SendToPartner);
    assert_eq!(prompts.local_opened(TUTORIAL_ID), LocalOpen::AlreadyShown);
    assert!(!prompts.partner_opened(TUTORIAL_ID));
    assert_eq!(prompts.next_to_open(true), None);
}

#[test]
fn a_partner_prompt_opened_here_is_not_shown_again() {
    let mut prompts = TutorialPrompts::new();
    assert!(prompts.partner_opened(TUTORIAL_ID));
    assert_eq!(prompts.next_to_open(true), Some(TUTORIAL_ID));
    assert!(!prompts.partner_opened(TUTORIAL_ID));
    assert_eq!(prompts.local_opened(TUTORIAL_ID), LocalOpen::AlreadyShown);
}

#[test]
fn a_queued_prompt_the_local_game_opened_meanwhile_is_dropped() {
    let mut prompts = TutorialPrompts::new();
    assert!(prompts.partner_opened(TUTORIAL_ID));
    assert_eq!(prompts.local_opened(TUTORIAL_ID), LocalOpen::SendToPartner);
    assert_eq!(prompts.next_to_open(true), None);
    assert_eq!(prompts.waiting_count(), 0);
}

#[test]
fn a_duplicate_message_queues_once_and_prompts_open_in_arrival_order() {
    let mut prompts = TutorialPrompts::new();
    assert!(prompts.partner_opened(TUTORIAL_ID));
    assert!(!prompts.partner_opened(TUTORIAL_ID));
    assert!(prompts.partner_opened(OTHER_TUTORIAL_ID));
    assert_eq!(prompts.next_to_open(true), Some(TUTORIAL_ID));
    assert_eq!(prompts.next_to_open(true), Some(OTHER_TUTORIAL_ID));
    assert_eq!(prompts.next_to_open(true), None);
}
