use rc_game::marker::{CONTROL_MASK_FIRST_RVA, EVENT_SCENE_BYTE_RVA};
use rc_game::move_log::{
    CURRENT_STATE_POINTER_OFFSET, GROUND_VELOCITY_OFFSET, MOVE_LOG_DURATION_MS,
    MOVE_LOG_INTERVAL_MS, MoveLineContext, MoveLogDue, MoveLogReason, MoveLogTrigger,
    MoveLogWindow, RATE_TABLE_OFFSET, action_set_for_seed, format_move_line, read_move_sample,
    state_name_for_rva,
};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const ACTOR: u64 = 0x2_0000_0000;
const STATE_OBJECT: u64 = 0x2_1000_0000;
const MOVE_STATE_RVA: u32 = 0xC9_3E98;

fn due() -> MoveLogDue {
    MoveLogDue {
        reason: MoveLogReason::PhaseLoad,
        seconds_in: 3,
        frames: 60,
        frame_ms_average: 16.7,
        frame_ms_longest: 40,
    }
}

fn memory_with_actor() -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put(
        ACTOR + CURRENT_STATE_POINTER_OFFSET,
        &STATE_OBJECT.to_le_bytes(),
    );
    memory.put(
        STATE_OBJECT,
        &(FAKE_MODULE_BASE + u64::from(MOVE_STATE_RVA)).to_le_bytes(),
    );
    let velocity: Vec<u8> = [3.0_f32, 0.0, 4.0]
        .iter()
        .flat_map(|lane| lane.to_le_bytes())
        .collect();
    memory.put(ACTOR + GROUND_VELOCITY_OFFSET, &velocity);
    memory.put(ACTOR + RATE_TABLE_OFFSET, &[0_u8; 0x5A0]);
    memory.put_at_rva(EVENT_SCENE_BYTE_RVA, &[0]);
    memory.put_at_rva(
        CONTROL_MASK_FIRST_RVA,
        &[0x481_u32, 0, 0, 0].map(u32::to_le_bytes).concat(),
    );
    memory
}

#[test]
fn a_new_player_pointer_is_a_phase_load() {
    let mut trigger = MoveLogTrigger::default();
    assert_eq!(trigger.observe(0, 0, false), None);
    assert_eq!(
        trigger.observe(7, 0x481, false),
        Some(MoveLogReason::PhaseLoad)
    );
    assert_eq!(trigger.observe(7, 0x481, false), None);
}

#[test]
fn the_mask_dropping_to_zero_is_a_hold_release() {
    let mut trigger = MoveLogTrigger::default();
    trigger.observe(7, 0xA0, false);
    assert_eq!(
        trigger.observe(7, 0, false),
        Some(MoveLogReason::ControlReleased)
    );
}

#[test]
fn a_closing_talk_window_is_reported_when_the_mask_stays_set() {
    let mut trigger = MoveLogTrigger::default();
    trigger.observe(7, 0x481, true);
    assert_eq!(
        trigger.observe(7, 0x481, false),
        Some(MoveLogReason::TalkClosed)
    );
}

#[test]
fn the_window_logs_once_a_second_and_stops_after_thirty_seconds() {
    let mut window = MoveLogWindow::default();
    assert!(window.on_tick(0).is_none());
    window.arm(MoveLogReason::PhaseLoad, 1000);
    assert!(window.on_tick(1016).is_some());
    assert!(window.on_tick(1500).is_none());
    let second = window
        .on_tick(1016 + MOVE_LOG_INTERVAL_MS)
        .expect("second line");
    assert_eq!(second.seconds_in, 1);
    assert!(window.on_tick(1000 + MOVE_LOG_DURATION_MS + 1).is_none());
    assert!(!window.is_armed());
}

#[test]
fn the_window_averages_frame_time_between_lines() {
    let mut window = MoveLogWindow::default();
    window.arm(MoveLogReason::TalkClosed, 0);
    window.on_tick(10);
    window.on_tick(20);
    window.on_tick(60);
    let report = window.on_tick(1010).expect("line due");
    assert_eq!(report.frames, 3);
    assert_eq!(report.frame_ms_longest, 950);
}

#[test]
fn a_sample_reads_state_speed_and_mask() {
    let sample = read_move_sample(&memory_with_actor(), ACTOR);
    assert_eq!(sample.state_rva, Some(MOVE_STATE_RVA));
    assert_eq!(sample.speed, Some(5.0));
    assert_eq!(sample.mask_slots, Some([0x481, 0, 0, 0]));
    assert_eq!(sample.event_scene, Some(false));
}

#[test]
fn the_rate_table_lists_only_values_that_are_not_one() {
    let mut memory = memory_with_actor();
    let mut table = vec![0_u8; 0x5A0];
    for at in (0..table.len()).step_by(8) {
        table[at..at + 4].copy_from_slice(&1.0_f32.to_le_bytes());
    }
    table[0x60 * 6 + 8..0x60 * 6 + 12].copy_from_slice(&10.0_f32.to_le_bytes());
    memory.put(ACTOR + RATE_TABLE_OFFSET, &table);
    let changes = read_move_sample(&memory, ACTOR)
        .rate_changes
        .expect("table read");
    assert_eq!(changes.len(), 1);
    assert_eq!(
        (changes[0].id, changes[0].channel, changes[0].value),
        (6, 1, 10.0)
    );
}

#[test]
fn unreadable_fields_print_a_question_mark() {
    let sample = read_move_sample(&FakeMemory::new(), ACTOR);
    let context = MoveLineContext {
        due: due(),
        action_set: "PlayerKaine",
        seed: "kaineE",
        talk_window_open: false,
        window_focused: true,
    };
    let line = format_move_line(&context, &sample);
    assert!(
        line.starts_with("move: phase load +3s action_set=PlayerKaine seed=kaineE state=? speed=?")
    );
    assert!(line.contains("focused=1 frame_ms=16.7 (max 40, 60 frames)"));
}

#[test]
fn a_full_line_names_the_state_and_the_changed_rates() {
    let sample = read_move_sample(&memory_with_actor(), ACTOR);
    let context = MoveLineContext {
        due: due(),
        action_set: "PlayerBoy",
        seed: "nierB",
        talk_window_open: true,
        window_focused: false,
    };
    let line = format_move_line(&context, &sample);
    assert!(line.contains("state=StateActionMove(0xC93E98) speed=5.00"));
    assert!(line.contains("mask=0x481/0x0/0x0/0x0"));
    assert!(line.contains("talk=1 focused=0"));
}

#[test]
fn seeds_map_to_action_sets_and_states_to_names() {
    assert_eq!(action_set_for_seed("kaineE"), "PlayerKaine");
    assert_eq!(action_set_for_seed("nierB"), "PlayerBoy");
    assert_eq!(action_set_for_seed("nierY"), "PlayerDefault");
    assert_eq!(state_name_for_rva(MOVE_STATE_RVA), "StateActionMove");
    assert_eq!(state_name_for_rva(1), "unknown");
}
