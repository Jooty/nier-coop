use rc_game::enemies::{BossHitLog, BossHitPlayer};

#[test]
fn at_most_four_lines_a_second() {
    let mut log = BossHitLog::new();
    let allowed = (0..10).filter(|i| log.allow_line(900 + i)).count();
    assert_eq!(allowed, 4);
    assert!(log.allow_line(2000));
}

#[test]
fn the_summary_counts_hits_and_damage_per_player_and_kind() {
    let mut log = BossHitLog::new();
    assert_eq!(log.record(0, BossHitPlayer::Host, 1, 10), None);
    assert_eq!(log.record(500, BossHitPlayer::Guest, 6, 100), None);
    assert_eq!(log.record(900, BossHitPlayer::Guest, 6, 120), None);
    let summary = log.record(2100, BossHitPlayer::Host, 1, 5).unwrap();
    assert_eq!(
        summary,
        "boss hits, last 2 s: host kind 1: 1 hits, damage 10; guest kind 6: 2 hits, damage 220"
    );
}

#[test]
fn the_next_window_starts_empty() {
    let mut log = BossHitLog::new();
    log.record(0, BossHitPlayer::Host, 1, 10);
    log.record(2100, BossHitPlayer::Host, 1, 5);
    let summary = log.record(4300, BossHitPlayer::Guest, 2, 1).unwrap();
    assert_eq!(
        summary,
        "boss hits, last 2 s: host kind 1: 1 hits, damage 5"
    );
}
