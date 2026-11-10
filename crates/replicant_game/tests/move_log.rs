use replicant_game::marker::{CONTROL_MASK_FIRST_RVA, EVENT_SCENE_BYTE_RVA};
use replicant_game::move_log::{
    CURRENT_STATE_POINTER_OFFSET, GROUND_VELOCITY_OFFSET, KEY_BIT_TABLE_RVA, KEY_SET_OFFSET,
    KEYBOARD_DEVICE_POINTER_RVA, MOVE_LOG_DURATION_MS, MOVE_LOG_INTERVAL_MS, MOVE_MODE_OFFSET,
    MoveLineContext, MoveLogDue, MoveLogReason, MoveLogTrigger, MoveLogWindow,
    RATE_TABLE_CHANNEL_STRIDE, RATE_TABLE_ID_STRIDE, RATE_TABLE_OFFSET,
    SPEC_DUMP_SECOND_DUMP_DELAY_MS, SPEC_TABLE_BYTE_COUNT, SPEC_TABLE_POINTER_OFFSET,
    SPEED_CAP_GIVE_UP_AFTER_RESETS, STATE_161F0_OFFSET, SpecDumpMoment, SpecDumpSchedule,
    SpeedCapLift, SpeedCapLog, SpeedCapStep, action_set_for_seed, format_move_line,
    format_rates_dump, format_spec_dump, format_speed_cap_log, input_block_getter, read_move_axis,
    read_move_keys, read_move_sample, read_rates_dump, read_spec_table, read_speed_cap, role_label,
    should_lift_speed_cap, speed_cap_address, state_name_for_rva, write_lifted_speed_cap,
};
use replicant_game::stay_together::FollowRole;

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
    assert!(
        format_move_line(&context, &sample)
            .contains("move_axis=0.00/1.00 move_keys=W--D ground=? mode6b778=?")
    );
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
    assert_eq!(schedule.observe(0, 0, 100), None);
    assert_eq!(schedule.observe(7, 1, 100), Some(SpecDumpMoment::PhaseLoad));
    assert_eq!(
        schedule.observe(7, 1, 100 + SPEC_DUMP_SECOND_DUMP_DELAY_MS - 1),
        None
    );
    assert_eq!(
        schedule.observe(7, 1, 100 + SPEC_DUMP_SECOND_DUMP_DELAY_MS),
        Some(SpecDumpMoment::FiveSecondsLater)
    );
    assert_eq!(
        schedule.observe(7, 1, 100 + 2 * SPEC_DUMP_SECOND_DUMP_DELAY_MS),
        None
    );
    assert_eq!(
        schedule.observe(8, 2, 20_000),
        Some(SpecDumpMoment::PhaseLoad)
    );
    assert_eq!(
        schedule.observe(8, 3, 20_001),
        Some(SpecDumpMoment::PhaseLoad)
    );
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
    assert_eq!(
        lines[37],
        "move spec +0x480: 288.0000 289.0000 290.0000 291.0000 292.0000 293.0000 294.0000 295.0000"
    );
    assert_eq!(lines[38], "move spec +0x4a0: 296.0000");
}

#[test]
fn a_null_or_unreadable_spec_table_logs_one_line_and_no_dump() {
    let mut memory = FakeMemory::new();
    memory.put(ACTOR + SPEC_TABLE_POINTER_OFFSET, &0_u64.to_le_bytes());
    let table = read_spec_table(&memory, ACTOR);
    let null = format_spec_dump(
        "host",
        "PlayerBoy",
        "nierB",
        SpecDumpMoment::FiveSecondsLater,
        table.as_ref(),
    );
    assert_eq!(
        null,
        [
            "move spec: role host, action_set PlayerBoy, seed nierB, when 5 s later, table null (no dump)"
        ]
    );
    memory.put(ACTOR + SPEC_TABLE_POINTER_OFFSET, &SPEC_TABLE.to_le_bytes());
    let table = read_spec_table(&memory, ACTOR);
    let unreadable = format_spec_dump(
        "host",
        "PlayerBoy",
        "nierB",
        SpecDumpMoment::PhaseLoad,
        table.as_ref(),
    );
    assert_eq!(unreadable.len(), 1);
    assert!(unreadable[0].ends_with("table 0x220000000, unreadable (no dump)"));
    let missing = format_spec_dump(
        "host",
        "PlayerBoy",
        "nierB",
        SpecDumpMoment::PhaseLoad,
        None,
    );
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
fn the_rates_dump_prints_every_slot_the_way_the_move_line_does() {
    let mut memory = memory_with_actor();
    memory.put(ACTOR + STATE_161F0_OFFSET, &5_i32.to_le_bytes());
    let slot = |id: u64, channel: u64| {
        ACTOR
            + RATE_TABLE_OFFSET
            + id * RATE_TABLE_ID_STRIDE as u64
            + channel * RATE_TABLE_CHANNEL_STRIDE as u64
    };
    memory.put(slot(5, 2), &0.01_f32.to_le_bytes());
    let dump = read_rates_dump(&memory, ACTOR);
    let lines = format_rates_dump("guest", "phase load", &dump);
    assert_eq!(
        lines[0],
        "move rates: role guest, when phase load, state161f0=5"
    );
    assert_eq!(lines.len(), 16);
    assert!(lines[6].starts_with("move rate 5: 5:0=0.00 5:1=0.00 5:2=0.01 5:3=0.00"));
    assert_eq!(read_speed_cap(&memory, ACTOR), Some(0.01));
}

#[test]
fn an_unreadable_rate_table_logs_one_line() {
    let lines = format_rates_dump(
        "host",
        "5 s later",
        &read_rates_dump(&FakeMemory::new(), ACTOR),
    );
    assert_eq!(
        lines,
        [
            "move rates: role host, when 5 s later, state161f0=?",
            "move rate table unreadable"
        ]
    );
}

#[test]
fn the_speed_cap_is_the_rate_five_slot_two_the_move_line_reads() {
    let mut memory = memory_with_actor();
    memory.put(speed_cap_address(ACTOR), &0.01_f32.to_le_bytes());
    let sample = read_move_sample(&memory, ACTOR);
    let changes = sample.rate_changes.unwrap();
    assert!(
        changes
            .iter()
            .any(|change| (change.id, change.channel, change.value) == (5, 2, 0.01))
    );
    assert!(write_lifted_speed_cap(&mut memory, ACTOR));
    assert_eq!(read_speed_cap(&memory, ACTOR), Some(1.0));
}

#[test]
fn only_a_swapped_kaine_outside_scenes_and_talks_has_a_capped_speed_lifted() {
    assert!(should_lift_speed_cap(0.01, true, false, false));
    assert!(!should_lift_speed_cap(0.05, true, false, false));
    assert!(!should_lift_speed_cap(1.0, true, false, false));
    assert!(!should_lift_speed_cap(0.01, false, false, false));
    assert!(!should_lift_speed_cap(0.01, true, true, false));
    assert!(!should_lift_speed_cap(0.01, true, false, true));
}

#[test]
fn the_lift_is_logged_once_per_phase_and_a_game_reset_once() {
    let mut lift = SpeedCapLift::default();
    let first = lift.observe(1, 0.01, true, false, false);
    assert_eq!(
        first,
        SpeedCapStep {
            write: true,
            log: Some(SpeedCapLog::Lifted { from: 0.01 })
        }
    );
    assert_eq!(
        lift.observe(1, 1.0, true, false, false),
        SpeedCapStep::default()
    );
    let reset = lift.observe(1, 0.01, true, false, false);
    assert_eq!(
        reset,
        SpeedCapStep {
            write: true,
            log: Some(SpeedCapLog::GameReset { to: 0.01 })
        }
    );
    assert_eq!(
        lift.observe(1, 0.02, true, false, false),
        SpeedCapStep {
            write: true,
            log: None
        }
    );
    let next_phase = lift.observe(2, 0.01, true, false, false);
    assert_eq!(next_phase.log, Some(SpeedCapLog::Lifted { from: 0.01 }));
}

#[test]
fn the_lift_gives_up_for_the_phase_after_too_many_game_resets() {
    let mut lift = SpeedCapLift::default();
    lift.observe(3, 0.0, true, false, false);
    for _ in 0..SPEED_CAP_GIVE_UP_AFTER_RESETS {
        assert!(lift.observe(3, 0.0, true, false, false).write);
    }
    let give_up = lift.observe(3, 0.0, true, false, false);
    assert_eq!(
        give_up,
        SpeedCapStep {
            write: false,
            log: Some(SpeedCapLog::GaveUp)
        }
    );
    assert_eq!(
        lift.observe(3, 0.0, true, false, false),
        SpeedCapStep::default()
    );
    assert!(lift.observe(4, 0.0, true, false, false).write);
}

#[test]
fn the_lift_stays_quiet_in_a_scene_and_the_log_lines_are_exact() {
    let mut lift = SpeedCapLift::default();
    assert_eq!(
        lift.observe(1, 0.0, true, true, false),
        SpeedCapStep::default()
    );
    assert_eq!(
        format_speed_cap_log(SpeedCapLog::Lifted { from: 0.01 }, 7),
        "kaine walk: lifted the house speed cap 5:2 from 0.01 to 1.0 (phase 7)"
    );
    assert_eq!(
        format_speed_cap_log(SpeedCapLog::GameReset { to: 0.0 }, 7),
        "kaine walk: the game reset 5:2 to 0.00"
    );
    assert_eq!(
        format_speed_cap_log(SpeedCapLog::GaveUp, 7),
        "kaine walk: gave up for phase 7"
    );
}
