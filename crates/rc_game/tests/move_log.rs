use rc_game::marker::{CONTROL_MASK_FIRST_RVA, EVENT_SCENE_BYTE_RVA};
use rc_game::move_log::{
    CURRENT_STATE_POINTER_OFFSET, GROUND_VELOCITY_OFFSET, KEY_BIT_TABLE_RVA, KEY_SET_OFFSET,
    KEYBOARD_DEVICE_POINTER_RVA, MOVE_LOG_DURATION_MS, MOVE_LOG_INTERVAL_MS, MoveLineContext,
    MoveLogDue, MoveLogReason, MoveLogTrigger, MoveLogWindow, RATE_TABLE_OFFSET,
    GLOBAL_SCALE_FIRST_FIELD_OFFSET, GLOBAL_SCALE_FUNCTION_RVA, GLOBAL_SCALE_SECOND_FIELD_OFFSET,
    MOVE_MODE_OFFSET, RATE_PAIR_FIRST_OFFSET, RATE_PAIR_SECOND_OFFSET, RATE_PAIR_STRIDE,
    STATE_161F0_OFFSET, format_rates_dump, read_rates_dump, SPEC_DUMP_SECOND_DUMP_DELAY_MS, SPEC_TABLE_BYTE_COUNT, SPEC_TABLE_POINTER_OFFSET,
    SpecDumpMoment, SpecDumpSchedule, action_set_for_seed, format_move_line, format_spec_dump, input_block_getter, read_move_axis, read_move_keys,
    read_move_sample, read_spec_table, role_label, state_name_for_rva,
};
use rc_game::stay_together::FollowRole;

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
        window_foreground: true,
        focus_losses: 0,
        key_clears: 0,
    };
    let line = format_move_line(&context, &sample);
    assert!(
        line.starts_with("move: phase load +3s action_set=PlayerKaine seed=kaineE state=? speed=?")
    );
    assert!(line.contains("focused=1 foreground=1 focus_losses=0 key_clears=0 move_axis=? move_keys=? ground=? mode6b778=? frame_ms=16.7 (max 40, 60 frames)"));
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
        window_foreground: false,
        focus_losses: 11,
        key_clears: 2,
    };
    let line = format_move_line(&context, &sample);
    assert!(line.contains("state=StateActionMove(0xC93E98) speed=5.00"));
    assert!(line.contains("mask=0x481/0x0/0x0/0x0"));
    assert!(line.contains("talk=1 focused=0 foreground=0 focus_losses=11 key_clears=2"));
}

#[test]
fn seeds_map_to_action_sets_and_states_to_names() {
    assert_eq!(action_set_for_seed("kaineE"), "PlayerKaine");
    assert_eq!(action_set_for_seed("nierB"), "PlayerBoy");
    assert_eq!(action_set_for_seed("nierY"), "PlayerDefault");
    assert_eq!(state_name_for_rva(MOVE_STATE_RVA), "StateActionMove");
    assert_eq!(state_name_for_rva(1), "unknown");
}

const KEYBOARD_DEVICE: u64 = 0x2_2000_0000;
const INPUT_BLOCK: u64 = 0x2_3000_0000;
const ACTOR_VFTABLE: u64 = 0x2_4000_0000;
const VK_W: u32 = 0x57;
const VK_D: u32 = 0x44;

fn memory_with_keyboard(w_bit: i32, d_bit: i32, key_words: [u64; 4]) -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(KEYBOARD_DEVICE_POINTER_RVA, &KEYBOARD_DEVICE.to_le_bytes());
    for (virtual_key, bit) in [(VK_W, w_bit), (0x41, 1), (0x53, 2), (VK_D, d_bit)] {
        memory.put_at_rva(KEY_BIT_TABLE_RVA + virtual_key * 4, &bit.to_le_bytes());
    }
    let words: Vec<u8> = key_words
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect();
    memory.put(KEYBOARD_DEVICE + KEY_SET_OFFSET, &words);
    memory
}

#[test]
fn held_move_keys_are_read_through_the_games_key_bit_table() {
    let memory = memory_with_keyboard(0, 70, [1, 1 << 6, 0, 0]);
    assert_eq!(read_move_keys(&memory), Some([true, false, false, true]));
}

#[test]
fn an_unmapped_key_makes_the_key_reading_unknown() {
    assert_eq!(read_move_keys(&memory_with_keyboard(-1, 70, [0; 4])), None);
    assert_eq!(read_move_keys(&FakeMemory::new()), None);
}

#[test]
fn the_move_axis_is_the_two_floats_of_the_input_block() {
    let mut memory = FakeMemory::new();
    memory.put(INPUT_BLOCK + 0x4C, &0.5_f32.to_le_bytes());
    memory.put(INPUT_BLOCK + 0x54, &(-1.0_f32).to_le_bytes());
    assert_eq!(read_move_axis(&memory, INPUT_BLOCK), Some([0.5, -1.0]));
    assert_eq!(read_move_axis(&FakeMemory::new(), INPUT_BLOCK), None);
}

#[test]
fn the_input_block_getter_must_lie_inside_the_game_code() {
    let mut memory = FakeMemory::new();
    memory.put(ACTOR, &ACTOR_VFTABLE.to_le_bytes());
    memory.put(
        ACTOR_VFTABLE + 0xCD8,
        &(FAKE_MODULE_BASE + 0x6B_BCD0).to_le_bytes(),
    );
    assert_eq!(
        input_block_getter(&memory, ACTOR),
        Some(FAKE_MODULE_BASE + 0x6B_BCD0)
    );
    memory.put(ACTOR_VFTABLE + 0xCD8, &0x1234_u64.to_le_bytes());
    assert_eq!(input_block_getter(&memory, ACTOR), None);
}

#[test]
fn the_line_prints_the_axis_and_the_held_keys() {
    let mut sample = read_move_sample(&memory_with_keyboard(0, 70, [1, 1 << 6, 0, 0]), ACTOR);
    sample.move_axis = Some([0.0, 1.0]);
    let context = MoveLineContext {
        due: due(),
        action_set: "PlayerBoy",
        seed: "nierB",
        talk_window_open: false,
        window_focused: true,
        window_foreground: true,
        focus_losses: 0,
        key_clears: 0,
    };
    assert!(format_move_line(&context, &sample).contains("move_axis=0.00/1.00 move_keys=W--D ground=? mode6b778=?"));
}

const SPEC_TABLE: u64 = 0x2_2000_0000;

fn memory_with_spec_table() -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put(ACTOR + SPEC_TABLE_POINTER_OFFSET, &SPEC_TABLE.to_le_bytes());
    let words: Vec<u8> = (0..SPEC_TABLE_BYTE_COUNT / 4)
        .flat_map(|index| (index as f32).to_le_bytes())
        .collect();
    memory.put(SPEC_TABLE, &words);
    memory
}

#[test]
fn the_spec_dump_comes_at_phase_load_and_five_seconds_later() {
    let mut schedule = SpecDumpSchedule::default();
    assert_eq!(schedule.observe(0, 100), None);
    assert_eq!(schedule.observe(7, 100), Some(SpecDumpMoment::PhaseLoad));
    assert_eq!(schedule.observe(7, 100 + SPEC_DUMP_SECOND_DUMP_DELAY_MS - 1), None);
    assert_eq!(
        schedule.observe(7, 100 + SPEC_DUMP_SECOND_DUMP_DELAY_MS),
        Some(SpecDumpMoment::FiveSecondsLater)
    );
    assert_eq!(schedule.observe(7, 100 + 2 * SPEC_DUMP_SECOND_DUMP_DELAY_MS), None);
    assert_eq!(schedule.observe(8, 20_000), Some(SpecDumpMoment::PhaseLoad));
}

#[test]
fn the_spec_dump_prints_a_header_and_eight_words_per_line() {
    let table = read_spec_table(&memory_with_spec_table(), ACTOR);
    let lines = format_spec_dump(
        role_label(FollowRole::Guest),
        "PlayerKaine",
        "kaineE",
        SpecDumpMoment::PhaseLoad,
        table.as_ref(),
    );
    assert_eq!(
        lines[0],
        "move spec: role guest, action_set PlayerKaine, seed kaineE, when phase load, table 0x220000000"
    );
    assert_eq!(
        lines[1],
        "move spec +0x000: 0.0000 1.0000 2.0000 3.0000 4.0000 5.0000 6.0000 7.0000"
    );
    assert_eq!(
        lines[2],
        "move spec +0x020: 8.0000 9.0000 10.0000 11.0000 12.0000 13.0000 14.0000 15.0000"
    );
    assert_eq!(lines.len(), 1 + 38);
    assert_eq!(lines[37], "move spec +0x480: 288.0000 289.0000 290.0000 291.0000 292.0000 293.0000 294.0000 295.0000");
    assert_eq!(lines[38], "move spec +0x4a0: 296.0000");
}

#[test]
fn a_null_or_unreadable_spec_table_logs_one_line_and_no_dump() {
    let mut memory = FakeMemory::new();
    memory.put(ACTOR + SPEC_TABLE_POINTER_OFFSET, &0_u64.to_le_bytes());
    let table = read_spec_table(&memory, ACTOR);
    let null = format_spec_dump("host", "PlayerBoy", "nierB", SpecDumpMoment::FiveSecondsLater, table.as_ref());
    assert_eq!(
        null,
        ["move spec: role host, action_set PlayerBoy, seed nierB, when 5 s later, table null (no dump)"]
    );
    memory.put(ACTOR + SPEC_TABLE_POINTER_OFFSET, &SPEC_TABLE.to_le_bytes());
    let table = read_spec_table(&memory, ACTOR);
    let unreadable = format_spec_dump("host", "PlayerBoy", "nierB", SpecDumpMoment::PhaseLoad, table.as_ref());
    assert_eq!(unreadable.len(), 1);
    assert!(unreadable[0].ends_with("table 0x220000000, unreadable (no dump)"));
    let missing = format_spec_dump("host", "PlayerBoy", "nierB", SpecDumpMoment::PhaseLoad, None);
    assert!(missing[0].ends_with("table pointer unreadable"));
}

#[test]
fn the_move_mode_and_ground_velocity_print_on_the_line() {
    let mut memory = memory_with_actor();
    memory.put(ACTOR + MOVE_MODE_OFFSET, &2_i32.to_le_bytes());
    let sample = read_move_sample(&memory, ACTOR);
    let context = MoveLineContext {
        due: due(),
        action_set: "PlayerBoy",
        seed: "nierB",
        talk_window_open: false,
        window_focused: true,
        window_foreground: true,
        focus_losses: 0,
        key_clears: 0,
    };
    assert!(format_move_line(&context, &sample).contains("ground=3.00/4.00 mode6b778=2 frame_ms="));
}

#[test]
fn the_rates_dump_prints_pairs_products_state_and_the_global_scale() {
    let mut memory = FakeMemory::new();
    let function = FAKE_MODULE_BASE + GLOBAL_SCALE_FUNCTION_RVA;
    let global = function + 0x1000;
    let mut code = vec![0x48, 0x8D, 0x05];
    code.extend_from_slice(&((global - (function + 7)) as i32).to_le_bytes());
    code.extend_from_slice(&[0xC3; 9]);
    memory.put(function, &code);
    memory.put(global + GLOBAL_SCALE_FIRST_FIELD_OFFSET, &2.0_f32.to_le_bytes());
    memory.put(global + GLOBAL_SCALE_SECOND_FIELD_OFFSET, &3.0_f32.to_le_bytes());
    memory.put(ACTOR + STATE_161F0_OFFSET, &5_i32.to_le_bytes());
    let floats = |values: [f32; 2]| values.map(f32::to_le_bytes).concat();
    memory.put(ACTOR + RATE_PAIR_FIRST_OFFSET + 5 * RATE_PAIR_STRIDE, &floats([1.0, 2.0]));
    memory.put(ACTOR + RATE_PAIR_SECOND_OFFSET + 5 * RATE_PAIR_STRIDE, &floats([0.5, 4.0]));
    let dump = read_rates_dump(&memory, ACTOR);
    assert_eq!(dump.global_fields, Some([2.0, 3.0]));
    assert_eq!(dump.pairs.len(), 15);
    assert_eq!(dump.pairs[4], None);
    let lines = format_rates_dump("guest", "phase load", &dump);
    assert!(lines[0].starts_with(
        "move rates: role guest, when phase load, state161f0=5, global_scale ptr=0x140562640 +0x48*+0x18=2.0000*3.0000=6.0000, scale_fn_code=48 8d 05"
    ));
    assert_eq!(lines[5], "move rate 4: ?");
    assert_eq!(lines[6], "move rate 5: 1.0000/2.0000 x 0.5000/4.0000 = 4.0000");
}
