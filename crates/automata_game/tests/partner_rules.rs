use automata_game::character::Character;
use automata_game::partner::{
    BodyFields, BodySnapshot, HelloRule, PartnerDrive, PositionCorrection, PositionCorrector,
    RemoteInput, RemoteStateIds, SendPacer, StateDecision, StateRule, button_mask_of,
    button_words_of, facing_error, facing_needs_write, is_never_started, partner_character_note,
    state_id_for_wire, wanted_partner,
};

fn ids(state: u16) -> RemoteStateIds {
    RemoteStateIds {
        state,
        variant: 0,
        b: 0,
        c: 0,
    }
}

fn input(held: u32, pressed: u32) -> BodySnapshot {
    BodySnapshot {
        held,
        pressed,
        ..BodySnapshot::default()
    }
}

#[test]
fn the_button_mask_has_one_bit_for_each_nonzero_word() {
    assert_eq!(button_mask_of([0; 8]), 0);
    assert_eq!(button_mask_of([0x14, 0, 3, 0, 0, 0, 0, 1]), 0b1000_0101);
}

#[test]
fn a_mask_bit_becomes_the_pressed_word_value_and_the_rest_zero() {
    assert_eq!(button_words_of(0b0000_0101), [0x14, 0, 0x14, 0, 0, 0, 0, 0]);
    assert_eq!(button_mask_of(button_words_of(0xA5)), 0xA5);
}

#[test]
fn a_state_id_outside_the_wire_range_is_sent_as_zero() {
    assert_eq!(state_id_for_wire(0x44), 0x44);
    assert_eq!(state_id_for_wire(-1), 0);
    assert_eq!(state_id_for_wire(0x1_0000), 0);
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
fn the_feed_keeps_the_newest_held_mask_speed_and_facing() {
    let mut remote = RemoteInput::default();
    remote.receive(&BodySnapshot {
        held: 1,
        button_mask: 0x80,
        speed: 3.0,
        world_facing: 1.5,
        ..BodySnapshot::default()
    });
    let feed = remote.take_feed();
    assert_eq!((feed.held, feed.button_mask), (1, 0x80));
    assert_eq!(feed.speed, 3.0);
    assert_eq!(feed.world_facing, 1.5);
    assert_eq!(feed.speed_squared(), 9.0);
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
fn the_states_that_crash_or_kill_are_never_started() {
    for state in [0x1B, 0x41, 0x68, 0x6A, 0x35, 0x36] {
        assert!(is_never_started(state));
    }
    assert!(!is_never_started(0x44));
}

#[test]
fn with_input_drive_a_state_is_started_after_six_differing_messages() {
    let mut rule = StateRule::new(PartnerDrive::Input);
    for _ in 0..5 {
        assert_eq!(rule.observe(1, ids(7)), StateDecision::Nothing);
    }
    assert_eq!(rule.observe(1, ids(7)), StateDecision::Start(ids(7)));
}

#[test]
fn with_input_drive_an_agreeing_message_starts_the_count_again() {
    let mut rule = StateRule::new(PartnerDrive::Input);
    for _ in 0..5 {
        rule.observe(1, ids(7));
    }
    assert_eq!(rule.observe(7, ids(7)), StateDecision::Nothing);
    for _ in 0..5 {
        assert_eq!(rule.observe(1, ids(7)), StateDecision::Nothing);
    }
    assert_eq!(rule.observe(1, ids(7)), StateDecision::Start(ids(7)));
}

#[test]
fn with_input_drive_the_same_remote_state_is_not_started_twice_in_a_row() {
    let mut rule = StateRule::new(PartnerDrive::Input);
    for _ in 0..6 {
        rule.observe(1, ids(7));
    }
    for _ in 0..20 {
        assert_eq!(rule.observe(1, ids(7)), StateDecision::Nothing);
    }
    for _ in 0..5 {
        rule.observe(1, ids(9));
    }
    assert_eq!(rule.observe(1, ids(9)), StateDecision::Start(ids(9)));
}

#[test]
fn a_never_started_state_is_blocked_not_started() {
    let mut rule = StateRule::new(PartnerDrive::Input);
    for _ in 0..5 {
        rule.observe(1, ids(0x68));
    }
    assert_eq!(rule.observe(1, ids(0x68)), StateDecision::Blocked(0x68));
    let mut by_state = StateRule::new(PartnerDrive::State);
    assert_eq!(by_state.observe(1, ids(0x36)), StateDecision::Blocked(0x36));
}

#[test]
fn with_state_drive_every_change_of_state_or_variant_starts_at_once() {
    let mut rule = StateRule::new(PartnerDrive::State);
    assert_eq!(rule.observe(1, ids(7)), StateDecision::Start(ids(7)));
    assert_eq!(rule.observe(7, ids(7)), StateDecision::Nothing);
    let new_variant = RemoteStateIds {
        variant: 2,
        ..ids(7)
    };
    assert_eq!(
        rule.observe(7, new_variant),
        StateDecision::Start(new_variant)
    );
    assert_eq!(rule.observe(1, new_variant), StateDecision::Nothing);
}

#[test]
fn with_state_drive_a_first_message_that_agrees_starts_nothing() {
    let mut rule = StateRule::new(PartnerDrive::State);
    assert_eq!(rule.observe(7, ids(7)), StateDecision::Nothing);
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
    assert!(!rule.should_send(false, Some(0x100)));
    assert!(rule.should_send(true, Some(0x100)));
    assert!(!rule.should_send(true, Some(0x100)));
    assert!(!rule.should_send(true, None));
    assert!(rule.should_send(true, Some(0x200)));
    assert!(!rule.should_send(true, Some(0x200)));
}

#[test]
fn a_reconnect_tells_the_character_again() {
    let mut rule = HelloRule::default();
    assert!(rule.should_send(true, None));
    assert!(!rule.should_send(true, Some(0x100)));
    assert!(!rule.should_send(false, Some(0x100)));
    assert!(rule.should_send(true, Some(0x100)));
}

#[test]
fn the_partner_is_the_link_setting_until_the_other_player_names_one() {
    assert_eq!(wanted_partner(Character::NineS, None), Character::NineS);
    assert_eq!(
        wanted_partner(Character::NineS, Some(Character::A2)),
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
fn the_drive_setting_reads_its_two_values() {
    assert_eq!(PartnerDrive::from_text("state"), Some(PartnerDrive::State));
    assert_eq!(PartnerDrive::from_text("input"), Some(PartnerDrive::Input));
    assert_eq!(PartnerDrive::from_text("pad"), None);
    assert_eq!(PartnerDrive::Input.label(), "input");
}
