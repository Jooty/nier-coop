use core::f32::consts::PI;

use automata_game::autopilot::{
    AUTOPILOT_CIRCLE_PERIOD_MS, AUTOPILOT_STICK_LENGTH, Autopilot, autopilot_feed,
};
use automata_game::partner::{HIGHEST_SPEED_SENT, STICK_LENGTH_ABOVE_WHICH_THE_GAME_WALKS};

#[test]
fn the_setting_is_off_unless_it_says_circle() {
    assert_eq!(Autopilot::default(), Autopilot::Off);
    assert_eq!(Autopilot::from_text("off"), Some(Autopilot::Off));
    assert_eq!(Autopilot::from_text("circle"), Some(Autopilot::Circle));
    assert_eq!(Autopilot::from_text("square"), None);
    assert_eq!(Autopilot::Circle.label(), "circle");
    assert_eq!(Autopilot::Off.label(), "off");
}

#[test]
fn nothing_is_fed_while_it_is_off_or_the_player_is_not_free() {
    assert_eq!(autopilot_feed(Autopilot::Off, true, 1_000), None);
    assert_eq!(autopilot_feed(Autopilot::Circle, false, 1_000), None);
}

#[test]
fn the_circle_is_a_slow_walk_whose_facing_goes_round_once_a_period() {
    let at = |now_ms| autopilot_feed(Autopilot::Circle, true, now_ms).expect("fed while free");
    assert_eq!(at(0).speed, AUTOPILOT_STICK_LENGTH);
    assert!((at(0).world_facing + PI).abs() < 1e-5);
    assert!(at(AUTOPILOT_CIRCLE_PERIOD_MS / 2).world_facing.abs() < 1e-5);
    assert!((at(AUTOPILOT_CIRCLE_PERIOD_MS).world_facing + PI).abs() < 1e-5);
    for now_ms in (0..AUTOPILOT_CIRCLE_PERIOD_MS * 2).step_by(777) {
        let feed = at(now_ms);
        assert!(feed.world_facing >= -PI && feed.world_facing <= PI);
        assert!(feed.speed > STICK_LENGTH_ABOVE_WHICH_THE_GAME_WALKS);
        assert!(feed.speed <= HIGHEST_SPEED_SENT);
    }
}
