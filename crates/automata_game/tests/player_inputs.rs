use automata_game::partner::{
    BodySnapshot, InputsBySlot, PARTNER_SILENCE_BEFORE_GIVE_BACK_MS, player_is_in_play,
};

fn holding(held: u32, speed: f32) -> BodySnapshot {
    BodySnapshot {
        held,
        pressed: held,
        speed,
        ..BodySnapshot::default()
    }
}

#[test]
fn inputs_of_two_senders_stay_apart() {
    let mut inputs = InputsBySlot::default();
    inputs.receive(2, &holding(0b01, 1.0));
    inputs.receive(3, &holding(0b10, 4.0));
    let third = inputs.take_feed(3);
    assert_eq!((third.held, third.pressed, third.speed), (0b10, 0b10, 4.0));
    let second = inputs.take_feed(2);
    assert_eq!(
        (second.held, second.pressed, second.speed),
        (0b01, 0b01, 1.0)
    );
    assert_eq!(inputs.take_feed(3).pressed, 0);
    inputs.receive(3, &holding(0b10, 4.0));
    inputs.drop_presses(2);
    assert_eq!(inputs.take_feed(2).pressed, 0);
    assert_eq!(inputs.take_feed(3).held, 0b10);
}

#[test]
fn a_sender_nobody_heard_feeds_nothing_and_a_forgotten_one_is_gone() {
    let mut inputs = InputsBySlot::default();
    assert_eq!(inputs.take_feed(5), Default::default());
    inputs.receive(5, &holding(1, 2.0));
    assert!(inputs.holds_input_of(5));
    inputs.forget(5);
    assert!(!inputs.holds_input_of(5));
    assert_eq!(inputs.take_feed(5), Default::default());
}

#[test]
fn a_player_is_in_play_until_its_states_go_silent() {
    assert!(player_is_in_play(None));
    assert!(player_is_in_play(Some(PARTNER_SILENCE_BEFORE_GIVE_BACK_MS)));
    assert!(!player_is_in_play(Some(
        PARTNER_SILENCE_BEFORE_GIVE_BACK_MS + 1
    )));
}
