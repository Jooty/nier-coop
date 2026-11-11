use replicant_game::effects::{PROBE_LOG_LINES_PER_WINDOW, PROBE_LOG_WINDOW_MS, ProbeLogLimiter};

#[test]
fn the_first_lines_of_a_window_are_admitted_with_nothing_dropped() {
    let mut limiter = ProbeLogLimiter::default();
    for _ in 0..PROBE_LOG_LINES_PER_WINDOW {
        assert_eq!(limiter.admit(1_000), Some(0));
    }
}

#[test]
fn lines_past_the_limit_are_dropped() {
    let mut limiter = ProbeLogLimiter::default();
    for _ in 0..PROBE_LOG_LINES_PER_WINDOW {
        limiter.admit(1_000);
    }
    assert_eq!(limiter.admit(1_001), None);
    assert_eq!(limiter.admit(1_002), None);
}

#[test]
fn the_next_window_reports_how_many_lines_were_dropped() {
    let mut limiter = ProbeLogLimiter::default();
    for _ in 0..PROBE_LOG_LINES_PER_WINDOW {
        limiter.admit(1_000);
    }
    for _ in 0..3 {
        limiter.admit(2_000);
    }
    assert_eq!(limiter.admit(1_000 + PROBE_LOG_WINDOW_MS), Some(3));
    assert_eq!(limiter.admit(1_000 + PROBE_LOG_WINDOW_MS), Some(0));
}
