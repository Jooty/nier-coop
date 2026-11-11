use replicant_game::dialogue::{
    CHOICE_FOLLOW_GIVES_THE_GUEST_ITS_BUTTONS_AFTER_MS, CHOICE_SELECTOR_AXIS_HORIZONTAL,
    CHOICE_SELECTOR_AXIS_OFFSET, CHOICE_SELECTOR_AXIS_VERTICAL,
    CHOICE_SELECTOR_CANCEL_INDEX_OFFSET, CHOICE_SELECTOR_HIGHLIGHTED_INDEX_OFFSET,
    CHOICE_SELECTOR_OPTION_COUNT_OFFSET, ChoiceFollowInput, ChoiceFollower, ChoiceHoverTracker,
    ChoicePrompt, ChoiceStep, FollowClock, FollowedPromptFrame, OwnChoiceMarker,
    PartnerChoiceHover, PromptPick, TALK_PLAYER_CHOICE_SELECTOR_OFFSET,
    TALK_PLAYER_PROMPT_KIND_OFFSET, TALK_PLAYER_TALK_LIST_OPTION_COUNT_OFFSET,
    TALK_UPDATE_INPUT_BYTES, TALK_UPDATE_INPUT_ENABLED_OFFSET,
    TALK_UPDATE_INPUT_HORIZONTAL_NEXT_OFFSETS, TALK_UPDATE_INPUT_VERTICAL_NEXT_OFFSETS,
    TALK_UPDATE_INPUT_VERTICAL_PREVIOUS_OFFSETS, TALK_WINDOW_PLAYER_OFFSET, TALK_WINDOW_RVA,
    followed_prompt_frame, host_answer_fits, marker_move, prompt_pick, read_choice_prompt,
    write_highlighted_option,
};
use replicant_proto::{ChoiceHover, DialogueLine, TalkChoice, TalkChoiceKind};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const PROMPT_TALK: u32 = 1089;
const PROMPT_LINE: u16 = 0;
const WINDOW_STATE_RUNS_THE_PLAYER: i32 = 2;
const PLAYER_STATE_WAITS_BEFORE_THE_LIST: i32 = 6;
const PLAYER_STATE_TALK_LIST: i32 = 7;
const PLAYER_STATE_TEXT: i32 = 4;
const PROMPT_KIND_TALK_LIST: i32 = 3;
const CANCEL_INDEX: i32 = 5;

fn player() -> u64 {
    FAKE_MODULE_BASE + u64::from(TALK_WINDOW_RVA) + TALK_WINDOW_PLAYER_OFFSET
}

fn selector() -> u64 {
    player() + TALK_PLAYER_CHOICE_SELECTOR_OFFSET
}

fn prompt_memory(player_state: i32, option_count: i32, highlighted: i32) -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(TALK_WINDOW_RVA, &WINDOW_STATE_RUNS_THE_PLAYER.to_le_bytes());
    memory.put(player(), &player_state.to_le_bytes());
    memory.put(
        player() + TALK_PLAYER_PROMPT_KIND_OFFSET,
        &PROMPT_KIND_TALK_LIST.to_le_bytes(),
    );
    memory.put(
        player() + TALK_PLAYER_TALK_LIST_OPTION_COUNT_OFFSET,
        &option_count.to_le_bytes(),
    );
    memory.put(
        selector() + CHOICE_SELECTOR_OPTION_COUNT_OFFSET,
        &option_count.to_le_bytes(),
    );
    memory.put(
        selector() + CHOICE_SELECTOR_AXIS_OFFSET,
        &CHOICE_SELECTOR_AXIS_VERTICAL.to_le_bytes(),
    );
    memory.put(
        selector() + CHOICE_SELECTOR_HIGHLIGHTED_INDEX_OFFSET,
        &highlighted.to_le_bytes(),
    );
    memory.put(
        selector() + CHOICE_SELECTOR_CANCEL_INDEX_OFFSET,
        &CANCEL_INDEX.to_le_bytes(),
    );
    memory
}

fn prompt_line() -> DialogueLine {
    DialogueLine {
        talk_id: PROMPT_TALK,
        line: PROMPT_LINE,
    }
}

fn hover(index: u8) -> ChoiceHover {
    ChoiceHover {
        talk_id: PROMPT_TALK,
        line: PROMPT_LINE,
        index,
    }
}

fn host_hover_on(memory: &FakeMemory, index: u8) -> (Option<ChoicePrompt>, Option<u8>) {
    let prompt = read_choice_prompt(memory);
    let mut partner = PartnerChoiceHover::new();
    partner.receive(hover(index));
    let option_count = prompt.map_or(0, |prompt| prompt.option_count);
    (prompt, partner.on_prompt(Some(prompt_line()), option_count))
}

fn input_with(pressed_offsets: &[usize], enabled: bool) -> [u8; TALK_UPDATE_INPUT_BYTES] {
    let mut input = [0_u8; TALK_UPDATE_INPUT_BYTES];
    input[TALK_UPDATE_INPUT_ENABLED_OFFSET] = u8::from(enabled);
    for offset in pressed_offsets {
        input[*offset] = 1;
    }
    input
}

#[test]
fn the_prompt_is_read_from_the_talk_player_not_from_the_window_state() {
    let memory = prompt_memory(PLAYER_STATE_TALK_LIST, 2, 1);
    assert_eq!(
        read_choice_prompt(&memory),
        Some(ChoicePrompt {
            kind: TalkChoiceKind::TalkList,
            on_list: true,
            option_count: 2,
            highlighted: Some(1),
            axis: CHOICE_SELECTOR_AXIS_VERTICAL,
        })
    );
    let waiting = prompt_memory(PLAYER_STATE_WAITS_BEFORE_THE_LIST, 2, 0);
    assert_eq!(
        read_choice_prompt(&waiting).map(|prompt| prompt.on_list),
        Some(false)
    );
    assert_eq!(
        read_choice_prompt(&prompt_memory(PLAYER_STATE_TEXT, 2, 0)),
        None
    );
    let mut closed = prompt_memory(PLAYER_STATE_TALK_LIST, 2, 0);
    closed.put_at_rva(TALK_WINDOW_RVA, &0_i32.to_le_bytes());
    assert_eq!(read_choice_prompt(&closed), None);
}

#[test]
fn a_list_that_cannot_be_read_still_leaves_the_prompt_open_for_the_hosts_answer() {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(TALK_WINDOW_RVA, &WINDOW_STATE_RUNS_THE_PLAYER.to_le_bytes());
    memory.put(player(), &PLAYER_STATE_TALK_LIST.to_le_bytes());
    let prompt = read_choice_prompt(&memory).unwrap();
    assert_eq!(prompt.kind, TalkChoiceKind::TalkList);
    assert_eq!((prompt.option_count, prompt.highlighted), (0, None));
    let mut partner = PartnerChoiceHover::new();
    partner.receive(hover(0));
    let host_hover = partner.on_prompt(Some(prompt_line()), prompt.option_count);
    assert_eq!(host_hover, None);
    assert_eq!(
        followed_prompt_frame(true, Some(prompt), host_hover, false).highlight,
        None
    );
    assert_eq!(
        OwnChoiceMarker::new().step(Some(prompt_line()), prompt.option_count, 0, 1),
        None
    );
    let odd_count = prompt_memory(PLAYER_STATE_TALK_LIST, 9000, 0);
    assert_eq!(read_choice_prompt(&odd_count).unwrap().option_count, 0);
}

#[test]
fn a_guest_that_follows_the_host_cannot_confirm_or_cancel() {
    assert_eq!(prompt_pick(1, true, None), PromptPick::Nothing);
    assert_eq!(prompt_pick(CANCEL_INDEX, true, None), PromptPick::Nothing);
    assert_eq!(prompt_pick(-1, true, None), PromptPick::Nothing);
    assert_eq!(prompt_pick(0, true, Some(1)), PromptPick::Host(1));
    assert_eq!(prompt_pick(-1, true, Some(1)), PromptPick::Host(1));
    let prompt = read_choice_prompt(&prompt_memory(PLAYER_STATE_TALK_LIST, 2, 0));
    assert!(followed_prompt_frame(true, prompt, None, false).buttons_off);
    assert!(followed_prompt_frame(true, None, None, false).buttons_off);
}

#[test]
fn a_player_whose_talk_is_their_own_picks_by_their_own_press() {
    assert_eq!(prompt_pick(1, false, None), PromptPick::Own(1));
    assert_eq!(prompt_pick(1, false, Some(0)), PromptPick::Own(1));
    assert_eq!(prompt_pick(-1, false, Some(0)), PromptPick::Host(0));
    assert_eq!(prompt_pick(-1, false, None), PromptPick::Nothing);
}

#[test]
fn the_hosts_highlighted_option_moves_the_guests_cursor() {
    let mut memory = prompt_memory(PLAYER_STATE_TALK_LIST, 3, 0);
    let (prompt, host_hover) = host_hover_on(&memory, 2);
    let frame = followed_prompt_frame(true, prompt, host_hover, false);
    assert_eq!(frame.highlight, Some(2));
    write_highlighted_option(&mut memory, 2).unwrap();
    assert_eq!(read_choice_prompt(&memory).unwrap().highlighted, Some(2));
    let (prompt, host_hover) = host_hover_on(&memory, 2);
    assert_eq!(
        followed_prompt_frame(true, prompt, host_hover, false).highlight,
        None
    );
}

#[test]
fn a_hover_for_another_line_or_past_the_list_moves_nothing() {
    let memory = prompt_memory(PLAYER_STATE_TALK_LIST, 2, 0);
    let (prompt, past_the_list) = host_hover_on(&memory, 2);
    assert_eq!(past_the_list, None);
    assert_eq!(
        followed_prompt_frame(true, prompt, past_the_list, false).highlight,
        None
    );
    let mut partner = PartnerChoiceHover::new();
    partner.receive(ChoiceHover {
        line: PROMPT_LINE + 1,
        ..hover(1)
    });
    assert_eq!(partner.on_prompt(Some(prompt_line()), 2), None);
    partner.receive(ChoiceHover {
        talk_id: PROMPT_TALK + 1,
        ..hover(1)
    });
    assert_eq!(partner.on_prompt(Some(prompt_line()), 2), None);
    partner.receive(hover(1));
    assert_eq!(partner.on_prompt(None, 2), None);
    assert_eq!(partner.on_prompt(Some(prompt_line()), 2), Some(1));
}

#[test]
fn a_hover_is_forgotten_when_the_partner_leaves_that_line() {
    let mut partner = PartnerChoiceHover::new();
    partner.receive(hover(1));
    partner.partner_moved_to(Some(prompt_line()));
    assert_eq!(partner.on_prompt(Some(prompt_line()), 2), Some(1));
    partner.partner_moved_to(Some(DialogueLine {
        talk_id: PROMPT_TALK + 2,
        line: 0,
    }));
    assert_eq!(partner.on_prompt(Some(prompt_line()), 2), None);
    partner.receive(hover(1));
    partner.partner_moved_to(None);
    assert_eq!(partner.on_prompt(Some(prompt_line()), 2), None);
}

#[test]
fn the_guests_copy_opens_its_list_when_the_host_is_on_theirs_or_has_answered() {
    let memory = prompt_memory(PLAYER_STATE_WAITS_BEFORE_THE_LIST, 2, 0);
    let (prompt, host_hover) = host_hover_on(&memory, 1);
    let frame = followed_prompt_frame(true, prompt, host_hover, false);
    assert_eq!(
        frame,
        FollowedPromptFrame {
            buttons_off: true,
            enter_list: true,
            highlight: None,
        }
    );
    assert!(!followed_prompt_frame(true, prompt, None, false).enter_list);
    assert!(followed_prompt_frame(true, prompt, None, true).enter_list);
    assert!(followed_prompt_frame(false, prompt, None, true).enter_list);
    let on_list = read_choice_prompt(&prompt_memory(PLAYER_STATE_TALK_LIST, 2, 0));
    assert!(!followed_prompt_frame(true, on_list, Some(1), true).enter_list);
}

#[test]
fn the_guests_own_buttons_move_a_marker_that_starts_on_the_hosts_option() {
    let mut marker = OwnChoiceMarker::new();
    assert_eq!(marker.step(Some(prompt_line()), 3, 1, 0), Some(1));
    assert_eq!(marker.step(Some(prompt_line()), 3, 0, 1), Some(2));
    assert_eq!(marker.step(Some(prompt_line()), 3, 0, 1), Some(0));
    assert_eq!(marker.step(Some(prompt_line()), 3, 0, -1), Some(2));
    assert_eq!(marker.step(Some(prompt_line()), 3, 0, 0), Some(2));
    assert_eq!(marker.step(Some(prompt_line()), 2, 0, 0), Some(1));
    let next_prompt = DialogueLine {
        talk_id: PROMPT_TALK + 4,
        line: 0,
    };
    assert_eq!(marker.step(Some(next_prompt), 3, 9, 0), Some(2));
    assert_eq!(marker.step(None, 3, 0, 1), None);
    assert_eq!(marker.step(Some(prompt_line()), 3, 0, 0), Some(0));
}

#[test]
fn the_marker_reads_the_presses_along_the_lists_axis() {
    let next = input_with(&[TALK_UPDATE_INPUT_VERTICAL_NEXT_OFFSETS[1]], true);
    let previous = input_with(&[TALK_UPDATE_INPUT_VERTICAL_PREVIOUS_OFFSETS[3]], true);
    assert_eq!(marker_move(CHOICE_SELECTOR_AXIS_VERTICAL, &next), 1);
    assert_eq!(marker_move(CHOICE_SELECTOR_AXIS_VERTICAL, &previous), -1);
    assert_eq!(marker_move(CHOICE_SELECTOR_AXIS_HORIZONTAL, &next), 0);
    let sideways = input_with(&[TALK_UPDATE_INPUT_HORIZONTAL_NEXT_OFFSETS[0]], true);
    assert_eq!(marker_move(CHOICE_SELECTOR_AXIS_HORIZONTAL, &sideways), 1);
    assert_eq!(marker_move(CHOICE_SELECTOR_AXIS_VERTICAL, &sideways), 0);
    let both = input_with(
        &[
            TALK_UPDATE_INPUT_VERTICAL_NEXT_OFFSETS[0],
            TALK_UPDATE_INPUT_VERTICAL_PREVIOUS_OFFSETS[0],
        ],
        true,
    );
    assert_eq!(marker_move(CHOICE_SELECTOR_AXIS_VERTICAL, &both), 0);
    let input_off = input_with(&[TALK_UPDATE_INPUT_VERTICAL_NEXT_OFFSETS[0]], false);
    assert_eq!(marker_move(CHOICE_SELECTOR_AXIS_VERTICAL, &input_off), 0);
    let mut held_only = input_with(&[], true);
    held_only[TALK_UPDATE_INPUT_VERTICAL_NEXT_OFFSETS[0]] = 2;
    assert_eq!(marker_move(CHOICE_SELECTOR_AXIS_VERTICAL, &held_only), 0);
    assert_eq!(marker_move(7, &next), 0);
    assert_eq!(marker_move(CHOICE_SELECTOR_AXIS_VERTICAL, &[]), 0);
}

#[test]
fn a_hover_is_sent_when_it_changes_and_again_when_the_partner_joins() {
    let mut tracker = ChoiceHoverTracker::new();
    assert_eq!(tracker.observe(Some(hover(0)), true), Some(hover(0)));
    assert_eq!(tracker.observe(Some(hover(0)), true), None);
    assert_eq!(tracker.observe(Some(hover(1)), true), Some(hover(1)));
    assert_eq!(tracker.observe(None, true), None);
    assert_eq!(tracker.observe(Some(hover(1)), true), Some(hover(1)));
    assert_eq!(tracker.observe(Some(hover(1)), false), None);
    assert_eq!(tracker.observe(Some(hover(0)), false), None);
    assert_eq!(tracker.observe(Some(hover(0)), true), Some(hover(0)));
}

#[test]
fn with_the_host_gone_the_guest_has_its_buttons_and_nothing_is_mirrored() {
    let memory = prompt_memory(PLAYER_STATE_TALK_LIST, 2, 0);
    let (prompt, host_hover) = host_hover_on(&memory, 1);
    assert_eq!(
        followed_prompt_frame(false, prompt, host_hover, false),
        FollowedPromptFrame::default()
    );
    assert_eq!(prompt_pick(1, false, None), PromptPick::Own(1));
    let mut partner = PartnerChoiceHover::new();
    partner.receive(hover(1));
    partner.forget();
    assert_eq!(partner.on_prompt(Some(prompt_line()), 2), None);
    let mut marker = OwnChoiceMarker::new();
    marker.step(Some(prompt_line()), 2, 1, 0);
    marker.forget();
    assert_eq!(marker.step(None, 2, 0, 0), None);
}

#[test]
fn the_hosts_answer_fits_a_listed_option_or_the_prompts_cancel_index() {
    let memory = prompt_memory(PLAYER_STATE_TALK_LIST, 2, 0);
    let answer = |kind, index| TalkChoice {
        talk_id: PROMPT_TALK,
        line: PROMPT_LINE,
        kind,
        index,
        target: 0,
    };
    assert!(host_answer_fits(
        &memory,
        &answer(TalkChoiceKind::TalkList, 1)
    ));
    assert!(!host_answer_fits(
        &memory,
        &answer(TalkChoiceKind::TalkList, 2)
    ));
    assert!(host_answer_fits(
        &memory,
        &answer(TalkChoiceKind::TalkList, CANCEL_INDEX as u8)
    ));
    assert!(host_answer_fits(
        &memory,
        &answer(TalkChoiceKind::MenuList, 40)
    ));
    assert!(!host_answer_fits(
        &FakeMemory::new(),
        &answer(TalkChoiceKind::TalkList, 0)
    ));
}

#[test]
fn the_wait_for_the_hosts_answer_stands_still_under_the_guests_menu() {
    let mut clock = FollowClock::new();
    assert_eq!(clock.now(1000, false), 1000);
    assert_eq!(clock.now(1500, true), 1000);
    assert_eq!(clock.now(9000, true), 1000);
    assert_eq!(clock.now(9100, false), 1100);
    let mut clock = FollowClock::new();
    let mut follower = ChoiceFollower::new();
    let host_answer = TalkChoice {
        talk_id: PROMPT_TALK,
        line: PROMPT_LINE,
        kind: TalkChoiceKind::TalkList,
        index: 1,
        target: 1091,
    };
    follower.receive(host_answer, clock.now(0, true));
    let at_the_prompt = |now_ms| ChoiceFollowInput {
        follows_host: true,
        own_talk: Some(prompt_line()),
        prompt: Some(TalkChoiceKind::TalkList),
        now_ms,
    };
    assert_eq!(
        follower.step(at_the_prompt(clock.now(10, true))),
        Some(ChoiceStep::Take(host_answer))
    );
    let long_under_the_menu = 4 * CHOICE_FOLLOW_GIVES_THE_GUEST_ITS_BUTTONS_AFTER_MS;
    assert_eq!(
        follower.step(at_the_prompt(clock.now(long_under_the_menu, true))),
        None
    );
    assert!(follower.is_waiting());
    assert_eq!(
        follower.step(at_the_prompt(clock.now(long_under_the_menu + 100, false))),
        None
    );
    assert!(follower.is_waiting());
}
