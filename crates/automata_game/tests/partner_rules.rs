use automata_game::address::{
    CONTROLLER_UPDATE_INPUT_CALLS, PROCESS_BUTTONS_FUNCTION, STICK_UPDATE_FUNCTION,
};
use automata_game::character::Character;
use automata_game::exe::ExeVariant;
use automata_game::partner::{
    ACTION_UPDATE_FUNCTION_START, BodyFields, BodySnapshot, CameraKept, DEFAULT_PARTNER_DRIVE,
    GiveBack, HIGHEST_SPEED_SENT, HelloRule, HeroIdentity, PARTNER_SILENCE_BEFORE_GIVE_BACK_MS,
    PartnerDrive, PositionCorrection, PositionCorrector, RecordedStart, RemoteInput, RemoteState,
    RemoteStateIds, SendPacer, StateDecision, StateRule, body_state_is_plain,
    button_words_after_presses, code_starts_with, controller_calls_match, facing_error,
    facing_needs_write, gives_fields_back, is_never_started, is_started_on_the_body,
    partner_character_note, partner_is_in_play, process_buttons_stores_match, speed_for_wire,
    state_id_for_wire, wanted_partner,
};

const IDLE: i32 = 0;
const WALK: u16 = 1;
const RUN: u16 = 2;
const A_STATE_NOT_ON_THE_ALLOW_LIST: u16 = 7;
const PRESS_TIMER_START: u32 = 0x14;

fn ids(state: u16) -> RemoteStateIds {
    RemoteStateIds {
        state,
        variant: 0,
        b: 0,
        c: 0,
    }
}

fn unrecorded(state: u16) -> RemoteState {
    RemoteState {
        state,
        last_start: None,
    }
}

fn recorded(state: u16, count: u32, variant: u32) -> RemoteState {
    RemoteState {
        state,
        last_start: Some(RecordedStart {
            count,
            ids: RemoteStateIds {
                state,
                variant,
                b: 5,
                c: 6,
            },
        }),
    }
}

fn hero(handle: u32, character: Character) -> Option<HeroIdentity> {
    Some(HeroIdentity {
        handle,
        character: Some(character),
    })
}

fn controller_calls(variant: ExeVariant) -> [u8; 26] {
    let site = CONTROLLER_UPDATE_INPUT_CALLS.rva(variant);
    let to_buttons = PROCESS_BUTTONS_FUNCTION.rva(variant).wrapping_sub(site + 8);
    let to_stick = STICK_UPDATE_FUNCTION.rva(variant).wrapping_sub(site + 19);
    let mut bytes = [0_u8; 26];
    bytes[0..3].copy_from_slice(&[0x48, 0x8B, 0xCB]);
    bytes[3] = 0xE8;
    bytes[4..8].copy_from_slice(&to_buttons.to_le_bytes());
    bytes[8..11].copy_from_slice(&[0x48, 0x8B, 0xD6]);
    bytes[11..14].copy_from_slice(&[0x48, 0x8B, 0xCB]);
    bytes[14] = 0xE8;
    bytes[15..19].copy_from_slice(&to_stick.to_le_bytes());
    bytes[19..21].copy_from_slice(&[0x83, 0x3D]);
    bytes[21..25].copy_from_slice(&[0x9B, 0x36, 0xF1, 0x00]);
    bytes[25] = 0x05;
    bytes
}

fn matches_for(variant: ExeVariant, bytes: &[u8; 26]) -> bool {
    controller_calls_match(
        bytes,
        CONTROLLER_UPDATE_INPUT_CALLS.rva(variant),
        PROCESS_BUTTONS_FUNCTION.rva(variant),
        STICK_UPDATE_FUNCTION.rva(variant),
    )
}

fn input(held: u32, pressed: u32) -> BodySnapshot {
    BodySnapshot {
        held,
        pressed,
        ..BodySnapshot::default()
    }
}

#[test]
fn a_press_sets_its_own_timer_and_leaves_the_running_ones_alone() {
    let counting_down = [0, 0, 9, 0, 3, 0, 0, 0];
    assert_eq!(
        button_words_after_presses(counting_down, 0b1000_1000, PRESS_TIMER_START),
        [0, 0, 9, PRESS_TIMER_START, 3, 0, 0, PRESS_TIMER_START]
    );
    assert_eq!(
        button_words_after_presses(counting_down, 0, PRESS_TIMER_START),
        counting_down
    );
}

#[test]
fn the_first_two_buttons_clear_each_others_timer_as_the_game_does() {
    let both_running = [7, 8, 0, 0, 0, 0, 0, 0];
    assert_eq!(
        button_words_after_presses(both_running, 0b01, PRESS_TIMER_START)[..2],
        [PRESS_TIMER_START, 0]
    );
    assert_eq!(
        button_words_after_presses(both_running, 0b10, PRESS_TIMER_START)[..2],
        [0, PRESS_TIMER_START]
    );
    assert_eq!(
        button_words_after_presses(both_running, 0b11, PRESS_TIMER_START)[..2],
        [0, PRESS_TIMER_START]
    );
}

#[test]
fn pressed_bits_above_the_eight_timers_set_no_timer() {
    assert_eq!(
        button_words_after_presses([0; 8], 0x700, PRESS_TIMER_START),
        [0; 8]
    );
}

#[test]
fn a_state_id_the_dispatcher_does_not_know_is_sent_as_zero() {
    assert_eq!(state_id_for_wire(0x44), 0x44);
    assert_eq!(state_id_for_wire(0xD3), 0xD3);
    assert_eq!(state_id_for_wire(0xD4), 0);
    assert_eq!(state_id_for_wire(-1), 0);
    assert_eq!(state_id_for_wire(0x1_0000), 0);
}

#[test]
fn the_speed_sent_is_a_stick_length_and_never_not_a_number() {
    assert_eq!(speed_for_wire(0.7), 0.7);
    assert_eq!(speed_for_wire(-1.0), 0.0);
    assert_eq!(speed_for_wire(900.0), HIGHEST_SPEED_SENT);
    assert_eq!(speed_for_wire(f32::NAN), 0.0);
    assert_eq!(speed_for_wire(f32::INFINITY), 0.0);
}

#[test]
fn a_recorded_start_of_an_unknown_state_is_not_sent() {
    assert_eq!(RecordedStart::of_call(4, 0xD4, 0, 0, 0), None);
    assert_eq!(RecordedStart::of_call(4, -1, 0, 0, 0), None);
    assert_eq!(
        RecordedStart::of_call(4, 2, 1, 5, 6),
        Some(RecordedStart {
            count: 4,
            ids: RemoteStateIds {
                state: 2,
                variant: 1,
                b: 5,
                c: 6
            }
        })
    );
}

#[test]
fn the_first_frame_sends_and_then_one_send_every_33_ms() {
    let mut pacer = SendPacer::default();
    assert_eq!(pacer.observe(0, 0), Some(0));
    assert_eq!(pacer.observe(16, 0), None);
    assert_eq!(pacer.observe(32, 0), None);
    assert_eq!(pacer.observe(33, 0), Some(0));
    assert_eq!(pacer.observe(50, 0), None);
}

#[test]
fn a_press_between_two_sends_goes_out_with_the_next_send_once() {
    let mut pacer = SendPacer::default();
    pacer.observe(0, 0);
    assert_eq!(pacer.observe(16, 0b10), None);
    assert_eq!(pacer.observe(34, 0b01), Some(0b11));
    assert_eq!(pacer.observe(70, 0), Some(0));
}

#[test]
fn pressed_bits_of_several_messages_are_fed_together_and_only_once() {
    let mut remote = RemoteInput::default();
    remote.receive(&input(0, 0b01));
    remote.receive(&input(0, 0b10));
    assert_eq!(remote.take_feed().pressed, 0b11);
    assert_eq!(remote.take_feed().pressed, 0);
}

#[test]
fn a_button_that_became_held_counts_as_pressed_when_its_edge_was_lost() {
    let mut remote = RemoteInput::default();
    remote.receive(&input(0b100, 0));
    let feed = remote.take_feed();
    assert_eq!(feed.held, 0b100);
    assert_eq!(feed.pressed, 0b100);
    remote.receive(&input(0b100, 0));
    assert_eq!(remote.take_feed().pressed, 0);
}

#[test]
fn the_feed_keeps_the_newest_held_buttons_speed_and_facing() {
    let mut remote = RemoteInput::default();
    remote.receive(&BodySnapshot {
        held: 1,
        speed: 3.0,
        world_facing: 1.5,
        ..BodySnapshot::default()
    });
    let feed = remote.take_feed();
    assert_eq!(feed.held, 1);
    assert_eq!(feed.speed, 3.0);
    assert_eq!(feed.world_facing, 1.5);
    assert_eq!(feed.speed_squared(), 9.0);
}

#[test]
fn a_feed_without_buttons_keeps_speed_and_facing() {
    let mut remote = RemoteInput::default();
    remote.receive(&BodySnapshot {
        held: 1,
        pressed: 2,
        speed: 0.5,
        world_facing: 1.5,
        ..BodySnapshot::default()
    });
    let feed = remote.take_feed().without_buttons();
    assert_eq!((feed.held, feed.pressed), (0, 0));
    assert_eq!((feed.speed, feed.world_facing), (0.5, 1.5));
}

#[test]
fn a_small_position_error_is_left_alone() {
    let mut corrector = PositionCorrector::default();
    assert_eq!(
        corrector.correct(0, [0.0, 0.0, 0.0], [0.05, 0.0, 0.05]),
        PositionCorrection::Leave
    );
}

#[test]
fn a_middle_position_error_is_nudged_by_a_fifth() {
    let mut corrector = PositionCorrector::default();
    assert_eq!(
        corrector.correct(0, [0.0, 0.0, 0.0], [0.5, 0.0, 0.0]),
        PositionCorrection::Nudge([0.1, 0.0, 0.0])
    );
}

#[test]
fn a_large_position_error_teleports_but_not_again_within_half_a_second() {
    let mut corrector = PositionCorrector::default();
    let far = [5.0, 0.0, 0.0];
    assert_eq!(
        corrector.correct(1_000, [0.0; 3], far),
        PositionCorrection::Teleport
    );
    assert_eq!(
        corrector.correct(1_100, [0.0; 3], far),
        PositionCorrection::Nudge([1.0, 0.0, 0.0])
    );
    assert_eq!(
        corrector.correct(1_500, [0.0; 3], far),
        PositionCorrection::Teleport
    );
}

#[test]
fn a_position_that_is_not_a_number_is_left_alone() {
    let mut corrector = PositionCorrector::default();
    assert_eq!(
        corrector.correct(0, [0.0; 3], [f32::NAN, 0.0, 0.0]),
        PositionCorrection::Leave
    );
}

#[test]
fn the_facing_error_takes_the_short_way_round() {
    assert!((facing_error(3.0, -3.0) - 0.283_185).abs() < 1e-4);
    assert!((facing_error(-3.0, 3.0) + 0.283_185).abs() < 1e-4);
    assert!(!facing_needs_write(3.0, -3.0));
    assert!(facing_needs_write(0.0, 2.0));
    assert!(!facing_needs_write(0.0, f32::NAN));
}

#[test]
fn the_states_that_crash_kill_or_hang_are_never_started() {
    for state in [0x1B, 0x41, 0x68, 0x6A, 0x35, 0x36, 0x3B] {
        assert!(is_never_started(state));
        assert!(!is_started_on_the_body(state));
    }
    assert!(!is_never_started(0x44));
}

#[test]
fn only_idle_walk_and_run_are_started_on_the_body() {
    for state in [0, 1, 2] {
        assert!(is_started_on_the_body(state));
    }
    for state in [3, 0xC, 0x44, 0xD3] {
        assert!(!is_started_on_the_body(state));
    }
}

#[test]
fn with_input_drive_a_state_is_started_after_six_differing_messages() {
    let mut rule = StateRule::new(PartnerDrive::Input);
    for _ in 0..5 {
        assert_eq!(rule.observe(IDLE, unrecorded(RUN)), StateDecision::Nothing);
    }
    assert_eq!(
        rule.observe(IDLE, unrecorded(RUN)),
        StateDecision::Start(ids(RUN))
    );
}

#[test]
fn with_input_drive_an_agreeing_message_starts_the_count_again() {
    let mut rule = StateRule::new(PartnerDrive::Input);
    for _ in 0..5 {
        rule.observe(IDLE, unrecorded(RUN));
    }
    assert_eq!(
        rule.observe(i32::from(RUN), unrecorded(RUN)),
        StateDecision::Nothing
    );
    for _ in 0..5 {
        assert_eq!(rule.observe(IDLE, unrecorded(RUN)), StateDecision::Nothing);
    }
    assert_eq!(
        rule.observe(IDLE, unrecorded(RUN)),
        StateDecision::Start(ids(RUN))
    );
}

#[test]
fn with_input_drive_the_same_remote_state_is_not_started_twice_in_a_row() {
    let mut rule = StateRule::new(PartnerDrive::Input);
    for _ in 0..6 {
        rule.observe(IDLE, unrecorded(RUN));
    }
    for _ in 0..20 {
        assert_eq!(rule.observe(IDLE, unrecorded(RUN)), StateDecision::Nothing);
    }
    for _ in 0..5 {
        rule.observe(IDLE, unrecorded(WALK));
    }
    assert_eq!(
        rule.observe(IDLE, unrecorded(WALK)),
        StateDecision::Start(ids(WALK))
    );
}

#[test]
fn after_a_teleport_the_same_state_is_started_again_in_both_drives() {
    for drive in [PartnerDrive::Input, PartnerDrive::State] {
        let mut rule = StateRule::new(drive);
        for _ in 0..6 {
            rule.observe(IDLE, unrecorded(RUN));
        }
        rule.forget_started();
        for _ in 0..5 {
            assert_eq!(rule.observe(IDLE, unrecorded(RUN)), StateDecision::Nothing);
        }
        assert_eq!(
            rule.observe(IDLE, unrecorded(RUN)),
            StateDecision::Start(ids(RUN))
        );
    }
}

#[test]
fn a_state_is_started_with_the_senders_own_start_arguments_not_its_live_phase() {
    let mut rule = StateRule::new(PartnerDrive::State);
    let running = recorded(RUN, 9, 0);
    assert_eq!(
        rule.observe(IDLE, running),
        StateDecision::Start(RemoteStateIds {
            state: RUN,
            variant: 0,
            b: 5,
            c: 6
        })
    );
}

#[test]
fn a_recorded_start_of_another_state_than_the_live_one_is_not_used() {
    let mut rule = StateRule::new(PartnerDrive::State);
    let walking_with_an_old_record = RemoteState {
        state: WALK,
        ..recorded(RUN, 9, 3)
    };
    assert_eq!(
        rule.observe(IDLE, walking_with_an_old_record),
        StateDecision::Start(ids(WALK))
    );
}

#[test]
fn with_state_drive_the_senders_start_counter_shows_a_new_start_of_the_same_state() {
    let mut rule = StateRule::new(PartnerDrive::State);
    assert!(matches!(
        rule.observe(IDLE, recorded(RUN, 9, 0)),
        StateDecision::Start(_)
    ));
    for _ in 0..20 {
        assert_eq!(
            rule.observe(i32::from(RUN), recorded(RUN, 9, 0)),
            StateDecision::Nothing
        );
    }
    assert!(matches!(
        rule.observe(i32::from(RUN), recorded(RUN, 10, 0)),
        StateDecision::Start(_)
    ));
}

#[test]
fn with_input_drive_a_new_start_of_a_state_the_body_is_in_starts_nothing() {
    let mut rule = StateRule::new(PartnerDrive::Input);
    rule.observe(i32::from(RUN), recorded(RUN, 9, 0));
    assert_eq!(
        rule.observe(i32::from(RUN), recorded(RUN, 10, 0)),
        StateDecision::Nothing
    );
}

#[test]
fn with_state_drive_a_body_that_left_the_state_is_started_again_after_six_messages() {
    let mut rule = StateRule::new(PartnerDrive::State);
    assert!(matches!(
        rule.observe(IDLE, recorded(RUN, 9, 0)),
        StateDecision::Start(_)
    ));
    rule.forget_started();
    for _ in 0..5 {
        assert_eq!(
            rule.observe(IDLE, recorded(RUN, 9, 0)),
            StateDecision::Nothing
        );
    }
    assert!(matches!(
        rule.observe(IDLE, recorded(RUN, 9, 0)),
        StateDecision::Start(_)
    ));
}

#[test]
fn a_state_that_is_not_on_the_allow_list_is_named_once_and_not_started() {
    let mut rule = StateRule::new(PartnerDrive::Input);
    for _ in 0..5 {
        rule.observe(IDLE, unrecorded(0x68));
    }
    assert_eq!(
        rule.observe(IDLE, unrecorded(0x68)),
        StateDecision::NotAllowed(0x68)
    );
    for _ in 0..20 {
        assert_eq!(rule.observe(IDLE, unrecorded(0x68)), StateDecision::Nothing);
    }
    let mut by_state = StateRule::new(PartnerDrive::State);
    assert_eq!(
        by_state.observe(IDLE, unrecorded(A_STATE_NOT_ON_THE_ALLOW_LIST)),
        StateDecision::NotAllowed(A_STATE_NOT_ON_THE_ALLOW_LIST)
    );
    assert_eq!(
        by_state.observe(IDLE, unrecorded(0x3B)),
        StateDecision::NotAllowed(0x3B)
    );
}

#[test]
fn with_state_drive_every_change_of_state_starts_at_once() {
    let mut rule = StateRule::new(PartnerDrive::State);
    assert_eq!(
        rule.observe(IDLE, unrecorded(WALK)),
        StateDecision::Start(ids(WALK))
    );
    assert_eq!(
        rule.observe(i32::from(WALK), unrecorded(WALK)),
        StateDecision::Nothing
    );
    assert_eq!(
        rule.observe(i32::from(WALK), unrecorded(RUN)),
        StateDecision::Start(ids(RUN))
    );
}

#[test]
fn with_state_drive_a_first_message_that_agrees_starts_nothing() {
    let mut rule = StateRule::new(PartnerDrive::State);
    assert_eq!(
        rule.observe(i32::from(RUN), unrecorded(RUN)),
        StateDecision::Nothing
    );
    let mut with_a_record = StateRule::new(PartnerDrive::State);
    assert_eq!(
        with_a_record.observe(i32::from(RUN), recorded(RUN, 9, 0)),
        StateDecision::Nothing
    );
}

#[test]
fn taking_over_clears_three_flag_bits_and_sets_the_table_values() {
    let before = BodyFields {
        flags: 0x0000_104D,
        player_controlled: 0,
        pad_enable: 1,
        script_enable: 1,
        computer_enable: 1,
        input_ignore_seconds: 2.0,
        run_speed_type: 1,
        dismissed: 1,
        health_floor_switch: 0,
    };
    assert_eq!(
        before.taken_over(),
        BodyFields {
            flags: 0x0000_1040,
            player_controlled: 1,
            pad_enable: 0,
            script_enable: 0,
            computer_enable: 0,
            input_ignore_seconds: 0.0,
            run_speed_type: 0,
            dismissed: 0,
            health_floor_switch: 1,
        }
    );
}

#[test]
fn a_body_whose_bit_six_stays_set_still_counts_as_computer_controlled() {
    let fields = BodyFields {
        flags: 0x41,
        player_controlled: 0,
        pad_enable: 0,
        script_enable: 0,
        computer_enable: 0,
        input_ignore_seconds: 0.0,
        run_speed_type: 0,
        dismissed: 0,
        health_floor_switch: 0,
    };
    assert!(fields.is_computer_controlled());
    assert!(fields.taken_over().is_computer_controlled());
    let cleared = BodyFields {
        flags: 0x1,
        ..fields
    };
    assert!(!cleared.taken_over().is_computer_controlled());
}

#[test]
fn the_character_is_told_on_connect_and_again_after_every_load() {
    let mut rule = HelloRule::default();
    let setting = Character::TwoB;
    let tell = |rule: &mut HelloRule, connected, hero| {
        rule.character_to_tell(connected, setting, hero, false)
    };
    assert_eq!(tell(&mut rule, false, hero(0x100, setting)), None);
    assert_eq!(tell(&mut rule, true, hero(0x100, setting)), Some(setting));
    assert_eq!(tell(&mut rule, true, hero(0x100, setting)), None);
    assert_eq!(tell(&mut rule, true, None), None);
    assert_eq!(tell(&mut rule, true, hero(0x200, setting)), Some(setting));
    assert_eq!(tell(&mut rule, true, hero(0x200, setting)), None);
}

#[test]
fn a_hero_that_comes_back_with_the_same_handle_is_told_again() {
    let mut rule = HelloRule::default();
    let setting = Character::TwoB;
    rule.character_to_tell(true, setting, hero(0x100, setting), false);
    assert_eq!(rule.character_to_tell(true, setting, None, false), None);
    assert_eq!(
        rule.character_to_tell(true, setting, hero(0x100, setting), false),
        Some(setting)
    );
}

#[test]
fn the_hello_names_the_body_the_hero_has_not_the_setting() {
    let mut rule = HelloRule::default();
    let setting = Character::TwoB;
    assert_eq!(
        rule.character_to_tell(true, setting, None, false),
        Some(setting)
    );
    assert_eq!(
        rule.character_to_tell(true, setting, hero(0x100, Character::A2), false),
        Some(Character::A2)
    );
    assert_eq!(
        rule.character_to_tell(true, setting, hero(0x100, Character::NineS), false),
        Some(Character::NineS)
    );
    assert_eq!(
        rule.character_to_tell(true, setting, hero(0x100, Character::NineS), false),
        None
    );
}

#[test]
fn the_hello_is_sent_once_more_when_the_other_game_is_first_heard() {
    let mut rule = HelloRule::default();
    let setting = Character::TwoB;
    let own_hero = hero(0x100, setting);
    assert_eq!(
        rule.character_to_tell(true, setting, own_hero, false),
        Some(setting)
    );
    assert_eq!(rule.character_to_tell(true, setting, own_hero, false), None);
    assert_eq!(
        rule.character_to_tell(true, setting, own_hero, true),
        Some(setting)
    );
    assert_eq!(rule.character_to_tell(true, setting, own_hero, true), None);
    rule.character_to_tell(false, setting, own_hero, false);
    assert_eq!(
        rule.character_to_tell(true, setting, own_hero, true),
        Some(setting)
    );
    assert_eq!(rule.character_to_tell(true, setting, own_hero, true), None);
}

#[test]
fn a_body_model_is_named_by_its_character() {
    assert_eq!(Character::from_model_id(0x10000), Some(Character::TwoB));
    assert_eq!(Character::from_model_id(0x10100), Some(Character::A2));
    assert_eq!(Character::from_model_id(0x10101), Some(Character::A2));
    assert_eq!(Character::from_model_id(0x10200), Some(Character::NineS));
    assert_eq!(Character::from_model_id(0x10203), Some(Character::NineS));
    assert_eq!(Character::from_model_id(0x10300), None);
}

#[test]
fn a_reconnect_tells_the_character_again() {
    let mut rule = HelloRule::default();
    let setting = Character::TwoB;
    let own_hero = hero(0x100, setting);
    assert_eq!(
        rule.character_to_tell(true, setting, own_hero, false),
        Some(setting)
    );
    assert_eq!(
        rule.character_to_tell(false, setting, own_hero, false),
        None
    );
    assert_eq!(
        rule.character_to_tell(true, setting, own_hero, false),
        Some(setting)
    );
}

#[test]
fn the_partner_is_the_link_setting_until_the_other_player_names_one() {
    assert_eq!(
        wanted_partner(Character::NineS, None, Some(Character::TwoB)),
        Character::NineS
    );
    assert_eq!(
        wanted_partner(Character::NineS, Some(Character::A2), Some(Character::TwoB)),
        Character::A2
    );
    assert_eq!(
        wanted_partner(Character::NineS, Some(Character::A2), None),
        Character::A2
    );
    assert_eq!(
        partner_character_note(Character::NineS, Character::NineS),
        None
    );
    assert!(
        partner_character_note(Character::NineS, Character::A2)
            .is_some_and(|line| line.contains("says it plays a2") && line.contains("said 9s"))
    );
}

#[test]
fn a_hello_that_names_our_own_bodys_character_is_ignored() {
    assert_eq!(
        wanted_partner(
            Character::NineS,
            Some(Character::TwoB),
            Some(Character::TwoB)
        ),
        Character::NineS
    );
}

#[test]
fn the_partner_body_is_never_the_character_of_our_own_body() {
    assert_eq!(
        wanted_partner(Character::NineS, None, Some(Character::NineS)),
        Character::TwoB
    );
    assert_eq!(
        wanted_partner(
            Character::TwoB,
            Some(Character::TwoB),
            Some(Character::TwoB)
        ),
        Character::NineS
    );
}

#[test]
fn the_other_player_is_in_play_only_while_its_game_sends_states() {
    assert!(!partner_is_in_play(true, None));
    assert!(partner_is_in_play(true, Some(0)));
    assert!(partner_is_in_play(
        true,
        Some(PARTNER_SILENCE_BEFORE_GIVE_BACK_MS)
    ));
    assert!(!partner_is_in_play(
        true,
        Some(PARTNER_SILENCE_BEFORE_GIVE_BACK_MS + 1)
    ));
    assert!(!partner_is_in_play(false, Some(0)));
}

#[test]
fn nothing_is_put_back_on_a_body_the_local_player_controls_now() {
    assert!(gives_fields_back(0x2200, Some(0x1100)));
    assert!(gives_fields_back(0x2200, None));
    assert!(!gives_fields_back(0x2200, Some(0x2200)));
}

#[test]
fn a_body_the_mod_made_is_given_back_with_the_following_run_speed_type() {
    let saved = BodyFields {
        flags: 0x41,
        player_controlled: 0,
        pad_enable: 0,
        script_enable: 1,
        computer_enable: 1,
        input_ignore_seconds: 0.0,
        run_speed_type: 0,
        dismissed: 0,
        health_floor_switch: 0,
    };
    let held = saved.taken_over();
    let adopted = saved.given_back(
        held,
        GiveBack {
            made_by_mod: false,
            mark_dismissed: false,
        },
    );
    assert_eq!(adopted, saved);
    let made = saved.given_back(
        held,
        GiveBack {
            made_by_mod: true,
            mark_dismissed: false,
        },
    );
    assert_eq!(made.run_speed_type, 1);
    assert_eq!(made.flags, saved.flags);
}

#[test]
fn an_adopted_body_that_fell_out_of_the_slot_is_given_back_marked_dismissed() {
    let saved = BodyFields {
        flags: 0x41,
        player_controlled: 0,
        pad_enable: 0,
        script_enable: 1,
        computer_enable: 1,
        input_ignore_seconds: 0.0,
        run_speed_type: 1,
        dismissed: 0,
        health_floor_switch: 0,
    };
    let given_back = saved.given_back(
        saved.taken_over(),
        GiveBack {
            made_by_mod: false,
            mark_dismissed: true,
        },
    );
    assert_eq!(given_back.dismissed, 1);
    let engine_dismissed = BodyFields {
        dismissed: 1,
        ..saved.taken_over()
    };
    let kept = saved.given_back(
        engine_dismissed,
        GiveBack {
            made_by_mod: false,
            mark_dismissed: false,
        },
    );
    assert_eq!(kept.dismissed, 1);
}

#[test]
fn a_function_start_matches_only_its_noted_bytes() {
    let mut found = ACTION_UPDATE_FUNCTION_START.to_vec();
    found.extend_from_slice(&[0x48, 0x8B]);
    assert!(code_starts_with(&found, &ACTION_UPDATE_FUNCTION_START));
    assert!(!code_starts_with(
        &found[..10],
        &ACTION_UPDATE_FUNCTION_START
    ));
    found[3] ^= 1;
    assert!(!code_starts_with(&found, &ACTION_UPDATE_FUNCTION_START));
}

#[test]
fn the_noted_call_pair_that_reaches_both_functions_matches_in_each_exe() {
    for variant in ExeVariant::ALL {
        assert!(matches_for(variant, &controller_calls(variant)));
    }
}

#[test]
fn the_input_addresses_are_the_ones_of_the_notes() {
    assert_eq!(PROCESS_BUTTONS_FUNCTION.rva(ExeVariant::Main), 0x52A790);
    assert_eq!(PROCESS_BUTTONS_FUNCTION.rva(ExeVariant::Compat), 0x524140);
    assert_eq!(STICK_UPDATE_FUNCTION.rva(ExeVariant::Main), 0x52AD90);
    assert_eq!(STICK_UPDATE_FUNCTION.rva(ExeVariant::Compat), 0x524740);
    assert_eq!(
        CONTROLLER_UPDATE_INPUT_CALLS.rva(ExeVariant::Main) + 19,
        0x501116
    );
    assert_eq!(
        CONTROLLER_UPDATE_INPUT_CALLS.rva(ExeVariant::Compat) + 19,
        0x4FABF6
    );
}

#[test]
fn a_call_that_goes_elsewhere_or_a_changed_byte_does_not_match() {
    let good = controller_calls(ExeVariant::Main);
    let mut other_target = good;
    other_target[15] ^= 0x10;
    assert!(!matches_for(ExeVariant::Main, &other_target));
    let mut other_code = good;
    other_code[9] = 0x89;
    assert!(!matches_for(ExeVariant::Main, &other_code));
    let mut other_mode = good;
    other_mode[25] = 0x04;
    assert!(!matches_for(ExeVariant::Main, &other_mode));
    assert!(!matches_for(ExeVariant::Compat, &good));
}

#[test]
fn the_button_function_is_known_by_its_two_held_flag_stores() {
    let mut start = [0x90_u8; 0x21];
    start[0x11..0x18].copy_from_slice(&[0x48, 0x89, 0x81, 0x38, 0x07, 0x00, 0x00]);
    start[0x1A..0x21].copy_from_slice(&[0x48, 0x89, 0x81, 0x40, 0x07, 0x00, 0x00]);
    assert!(process_buttons_stores_match(&start));
    assert!(!process_buttons_stores_match(&start[..0x20]));
    start[0x1D] = 0x48;
    assert!(!process_buttons_stores_match(&start));
}

#[test]
fn a_stopped_feed_keeps_only_the_facing() {
    let mut remote = RemoteInput::default();
    remote.receive(&BodySnapshot {
        held: 1,
        pressed: 1,
        speed: 3.0,
        world_facing: 1.5,
        ..BodySnapshot::default()
    });
    let stopped = remote.take_feed().stopped();
    assert_eq!((stopped.held, stopped.pressed), (0, 0));
    assert_eq!(stopped.speed, 0.0);
    assert_eq!(stopped.world_facing, 1.5);
}

#[test]
fn the_partner_is_driven_by_input_unless_the_setting_says_state() {
    assert_eq!(DEFAULT_PARTNER_DRIVE, PartnerDrive::Input);
}

#[test]
fn the_drive_setting_reads_its_two_values() {
    assert_eq!(PartnerDrive::from_text("state"), Some(PartnerDrive::State));
    assert_eq!(PartnerDrive::from_text("input"), Some(PartnerDrive::Input));
    assert_eq!(PartnerDrive::from_text("pad"), None);
    assert_eq!(PartnerDrive::Input.label(), "input");
}

#[test]
fn only_the_bit_a_camera_mode_request_clears_is_put_back() {
    let kept = CameraKept {
        lock_targets: [0, 0],
        mode: 7,
        word_zeroed_by_a_mode_request: 3,
        flag_word: 0x0000_8001,
        other_fields: [1, 2, 3],
    };
    assert_eq!(kept.flag_word_put_back(0x0000_0010), 0x0000_8010);
    let kept_clear = CameraKept {
        flag_word: 0x0000_0001,
        ..kept
    };
    assert_eq!(kept_clear.flag_word_put_back(0x0000_8010), 0x0000_0010);
}

#[test]
fn with_no_hero_the_character_told_last_stands() {
    let mut rule = HelloRule::default();
    let nine_s_hero = Some(HeroIdentity {
        handle: 0x100,
        character: Some(Character::NineS),
    });
    assert_eq!(
        rule.character_to_tell(true, Character::TwoB, nine_s_hero, false),
        Some(Character::NineS)
    );
    assert_eq!(
        rule.character_to_tell(true, Character::TwoB, None, false),
        None
    );
    let hero_after_the_load = Some(HeroIdentity {
        handle: 0x200,
        character: Some(Character::NineS),
    });
    assert_eq!(
        rule.character_to_tell(true, Character::TwoB, hero_after_the_load, false),
        Some(Character::NineS)
    );
}

#[test]
fn dropped_presses_do_not_reach_the_next_feed() {
    let mut input = RemoteInput::default();
    input.receive(&BodySnapshot {
        held: 0b1,
        pressed: 0b11,
        ..BodySnapshot::default()
    });
    input.drop_presses();
    let feed = input.take_feed();
    assert_eq!(feed.pressed, 0);
    assert_eq!(feed.held, 0b1);
}

#[test]
fn no_state_is_started_on_a_body_that_is_not_in_a_plain_state() {
    let mut rule = StateRule::new(PartnerDrive::Input);
    let idle = RemoteState {
        state: 0,
        last_start: None,
    };
    for _ in 0..20 {
        assert_eq!(
            rule.observe(0xB3, idle),
            StateDecision::BodyNotInAPlainState(0xB3)
        );
    }
    assert!(body_state_is_plain(0));
    assert!(body_state_is_plain(1));
    assert!(body_state_is_plain(2));
    assert!(!body_state_is_plain(0xB3));
    assert!(!body_state_is_plain(-1));
}

#[test]
fn a_body_back_in_a_plain_state_gets_states_again_after_the_usual_count() {
    let mut rule = StateRule::new(PartnerDrive::Input);
    let walking = RemoteState {
        state: 1,
        last_start: None,
    };
    assert_eq!(
        rule.observe(0x38, walking),
        StateDecision::BodyNotInAPlainState(0x38)
    );
    for _ in 0..5 {
        assert_eq!(rule.observe(0, walking), StateDecision::Nothing);
    }
    assert!(matches!(rule.observe(0, walking), StateDecision::Start(_)));
}
