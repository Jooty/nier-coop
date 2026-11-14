use automata_game::minute_line_budget::{MinuteLineBudget, held_back_line};

const TOPIC: &str = "enemies";

#[test]
fn a_quiet_minute_costs_nothing_and_notes_nothing() {
    let mut budget = MinuteLineBudget::new(3);
    for now_ms in [0, 10_000, 20_000] {
        let admission = budget.admit(now_ms, TOPIC);
        assert!(admission.allowed);
        assert_eq!(admission.held_back_note, None);
    }
    let next = budget.admit(60_000, TOPIC);
    assert!(next.allowed);
    assert_eq!(next.held_back_note, None);
}

#[test]
fn a_busy_minute_stops_at_the_budget() {
    let mut budget = MinuteLineBudget::new(3);
    let allowed = (0..10)
        .filter(|tick| budget.admit(1_000 + tick * 100, TOPIC).allowed)
        .count();
    assert_eq!(allowed, 3);
}

#[test]
fn the_next_minute_starts_fresh() {
    let mut budget = MinuteLineBudget::new(2);
    for _ in 0..5 {
        budget.admit(5_000, TOPIC);
    }
    assert!(budget.admit(60_000, TOPIC).allowed);
    assert!(budget.admit(61_000, TOPIC).allowed);
    assert!(!budget.admit(62_000, TOPIC).allowed);
}

#[test]
fn the_held_back_count_is_reported_once_with_the_next_minutes_first_line() {
    let mut budget = MinuteLineBudget::new(2);
    for _ in 0..5 {
        budget.admit(5_000, TOPIC);
    }
    let first = budget.admit(65_000, TOPIC);
    assert!(first.allowed);
    assert_eq!(first.held_back_note, Some(held_back_line(TOPIC, 3)));
    let second = budget.admit(66_000, TOPIC);
    assert_eq!(second.held_back_note, None);
}

#[test]
fn the_held_back_count_survives_a_quiet_gap_of_several_minutes() {
    let mut budget = MinuteLineBudget::new(1);
    for _ in 0..4 {
        budget.admit(0, TOPIC);
    }
    let after_gap = budget.admit(600_000, TOPIC);
    assert_eq!(after_gap.held_back_note, Some(held_back_line(TOPIC, 3)));
}

#[test]
fn the_note_names_the_topic_and_the_count() {
    assert_eq!(
        held_back_line("enemies: hits", 7),
        "enemies: hits: 7 more lines of this kind held back until the next minute"
    );
}
