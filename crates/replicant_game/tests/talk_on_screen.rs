use replicant_game::dialogue::{
    ScreenReport, TALK_DATA_LINE_COUNT_OFFSET, TALK_SCREEN_REPORT_AFTER_OPEN_MS,
    TALK_WIDGET_NODE_OFFSET, TALK_WINDOW_CURRENT_LINE_OFFSET, TALK_WINDOW_CURRENT_TALK_OFFSET,
    TALK_WINDOW_MAY_SHOW_OFFSET, TALK_WINDOW_PLAYER_OFFSET, TALK_WINDOW_RVA,
    TALK_WINDOW_WIDGET_OFFSET, TalkScreenVerdict, WIDGET_HANDLE_ENTRY_BYTES,
    WIDGET_HANDLE_ENTRY_GENERATION_OFFSET, WIDGET_HANDLE_TABLE_RVA, WIDGET_NODE_HANDLE_OFFSET,
    WIDGET_OBJECT_FLAGS_OFFSET, put_may_show_flag_back, read_talk_window_screen,
};
use replicant_game::marker::UI_DIALOG_STATE_RVA;

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const MIRRORED_TALK: u32 = 1223;
const MIRROR_OPENED_MS: u64 = 63_463_861;
const TALK_DATA: u64 = 0x3_0000_0000;
const WIDGET_NODE: u64 = 0x3_0001_0000;
const WIDGET_OBJECT: u64 = 0x3_0002_0000;
const WIDGET_HANDLE_SLOT: u32 = 7;
const WIDGET_HANDLE_GENERATION: u32 = 3;
const TALK_PLAYER_WAITING_FOR_A_PRESS: i32 = 4;

struct Window {
    state: i32,
    may_show: bool,
    main_menu: bool,
    drawn: bool,
    player_state: i32,
    line: i32,
    line_count: i32,
}

fn window_memory(window: &Window) -> FakeMemory {
    let mut memory = FakeMemory::new();
    let base = FAKE_MODULE_BASE + u64::from(TALK_WINDOW_RVA);
    memory.put(base, &window.state.to_le_bytes());
    memory.put(
        base + TALK_WINDOW_MAY_SHOW_OFFSET,
        &[u8::from(window.may_show)],
    );
    memory.put(
        base + TALK_WINDOW_PLAYER_OFFSET,
        &window.player_state.to_le_bytes(),
    );
    memory.put(
        base + TALK_WINDOW_CURRENT_TALK_OFFSET,
        &TALK_DATA.to_le_bytes(),
    );
    memory.put(
        base + TALK_WINDOW_CURRENT_LINE_OFFSET,
        &window.line.to_le_bytes(),
    );
    memory.put(
        TALK_DATA + TALK_DATA_LINE_COUNT_OFFSET,
        &window.line_count.to_le_bytes(),
    );
    memory.put_at_rva(
        UI_DIALOG_STATE_RVA,
        &i32::from(window.main_menu).to_le_bytes(),
    );
    memory.put(
        base + TALK_WINDOW_WIDGET_OFFSET + TALK_WIDGET_NODE_OFFSET,
        &WIDGET_NODE.to_le_bytes(),
    );
    let handle = (WIDGET_HANDLE_GENERATION << 16) | WIDGET_HANDLE_SLOT;
    memory.put(
        WIDGET_NODE + WIDGET_NODE_HANDLE_OFFSET,
        &handle.to_le_bytes(),
    );
    let entry = FAKE_MODULE_BASE
        + u64::from(WIDGET_HANDLE_TABLE_RVA)
        + u64::from(WIDGET_HANDLE_SLOT) * WIDGET_HANDLE_ENTRY_BYTES;
    memory.put(entry, &WIDGET_OBJECT.to_le_bytes());
    memory.put(
        entry + WIDGET_HANDLE_ENTRY_GENERATION_OFFSET,
        &WIDGET_HANDLE_GENERATION.to_le_bytes(),
    );
    memory.put(
        WIDGET_OBJECT + WIDGET_OBJECT_FLAGS_OFFSET,
        &[u8::from(window.drawn)],
    );
    memory
}

fn shown_window() -> Window {
    Window {
        state: 2,
        may_show: true,
        main_menu: false,
        drawn: true,
        player_state: TALK_PLAYER_WAITING_FOR_A_PRESS,
        line: 0,
        line_count: 1,
    }
}

#[test]
fn a_window_whose_may_show_flag_is_off_is_open_but_not_on_screen() {
    let memory = window_memory(&Window {
        may_show: false,
        drawn: false,
        ..shown_window()
    });
    let screen = read_talk_window_screen(&memory).unwrap();
    assert_eq!(screen.window_state, 2);
    assert_eq!(screen.verdict(), TalkScreenVerdict::HiddenByMayShowFlag);
    assert!(screen.describe().contains("may-show flag 0"));
}

#[test]
fn a_drawn_window_on_a_line_its_talk_has_is_on_screen() {
    let screen = read_talk_window_screen(&window_memory(&shown_window())).unwrap();
    assert_eq!(screen.widget_drawn, Some(true));
    assert_eq!(screen.line_count, Some(1));
    assert_eq!(screen.verdict(), TalkScreenVerdict::OnScreen);
}

#[test]
fn a_window_past_its_last_line_or_in_its_opening_state_shows_no_text() {
    for window in [
        Window {
            line: 1,
            ..shown_window()
        },
        Window {
            player_state: 0,
            ..shown_window()
        },
    ] {
        let screen = read_talk_window_screen(&window_memory(&window)).unwrap();
        assert_eq!(screen.verdict(), TalkScreenVerdict::NoTextYet);
    }
}

#[test]
fn a_main_menu_hides_the_window_before_the_flag_is_judged() {
    let screen = read_talk_window_screen(&window_memory(&Window {
        main_menu: true,
        may_show: false,
        ..shown_window()
    }))
    .unwrap();
    assert_eq!(screen.verdict(), TalkScreenVerdict::HiddenByMenu);
    assert!(!screen.may_show_flag_left_off(false));
}

#[test]
fn the_flag_counts_as_left_off_only_with_the_window_open_and_no_menu_up() {
    let hidden = Window {
        may_show: false,
        ..shown_window()
    };
    let screen = read_talk_window_screen(&window_memory(&hidden)).unwrap();
    assert!(screen.may_show_flag_left_off(false));
    assert!(!screen.may_show_flag_left_off(true));
    let closed = read_talk_window_screen(&window_memory(&Window { state: 0, ..hidden })).unwrap();
    assert!(!closed.may_show_flag_left_off(false));
}

#[test]
fn putting_the_flag_back_makes_the_window_show() {
    let mut memory = window_memory(&Window {
        may_show: false,
        ..shown_window()
    });
    put_may_show_flag_back(&mut memory).unwrap();
    let screen = read_talk_window_screen(&memory).unwrap();
    assert_eq!(screen.verdict(), TalkScreenVerdict::OnScreen);
}

#[test]
fn a_widget_handle_of_a_freed_object_reads_as_no_widget() {
    let mut memory = window_memory(&shown_window());
    let stale = ((WIDGET_HANDLE_GENERATION + 1) << 16) | WIDGET_HANDLE_SLOT;
    memory.put(
        WIDGET_NODE + WIDGET_NODE_HANDLE_OFFSET,
        &stale.to_le_bytes(),
    );
    assert_eq!(read_talk_window_screen(&memory).unwrap().widget_drawn, None);
}

#[test]
fn the_screen_is_reported_once_it_had_time_to_draw_and_again_when_it_changes() {
    let mut report = ScreenReport::new();
    let talk = Some(MIRRORED_TALK);
    let settle = TALK_SCREEN_REPORT_AFTER_OPEN_MS;
    let opened = MIRROR_OPENED_MS;
    let hidden = TalkScreenVerdict::HiddenByMayShowFlag;
    assert_eq!(report.observe(talk, hidden, opened, settle), None);
    assert_eq!(
        report.observe(talk, hidden, opened + settle - 1, settle),
        None
    );
    assert_eq!(
        report.observe(talk, hidden, opened + settle, settle),
        Some((MIRRORED_TALK, hidden))
    );
    assert_eq!(
        report.observe(talk, hidden, opened + settle + 16, settle),
        None
    );
    assert_eq!(
        report.observe(
            talk,
            TalkScreenVerdict::OnScreen,
            opened + settle + 32,
            settle
        ),
        Some((MIRRORED_TALK, TalkScreenVerdict::OnScreen))
    );
    assert_eq!(
        report.observe(None, TalkScreenVerdict::Closed, opened + 2000, settle),
        None
    );
    assert_eq!(report.observe(talk, hidden, opened + 2016, settle), None);
}
