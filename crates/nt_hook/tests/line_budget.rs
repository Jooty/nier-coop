use nt_hook::line_budget::{KeyedLineBudget, LineBudget};

const WINDOW_MS: u64 = 10_000;
const LINES_PER_WINDOW: u32 = 40;

#[test]
fn a_window_admits_its_lines_and_counts_the_rest() {
    let mut budget = LineBudget::counting_from_zero();
    assert!(budget.admit(0, 1000, 2));
    assert!(budget.admit(10, 1000, 2));
    assert!(!budget.admit(20, 1000, 2));
    assert!(!budget.admit(30, 1000, 2));
    assert_eq!(budget.take_refused(40, 1000), 2);
    assert_eq!(budget.take_refused(50, 1000), 0);
    assert!(budget.admit(1000, 1000, 2));
}

#[test]
fn the_first_lines_of_a_window_are_admitted_with_nothing_refused() {
    let mut budget = LineBudget::new();
    for _ in 0..LINES_PER_WINDOW {
        assert!(budget.admit(1_000, WINDOW_MS, LINES_PER_WINDOW));
    }
    assert_eq!(budget.take_refused(1_000, WINDOW_MS), 0);
}

#[test]
fn lines_past_the_limit_are_refused() {
    let mut budget = LineBudget::new();
    for _ in 0..LINES_PER_WINDOW {
        budget.admit(1_000, WINDOW_MS, LINES_PER_WINDOW);
    }
    assert!(!budget.admit(1_001, WINDOW_MS, LINES_PER_WINDOW));
    assert!(!budget.admit(1_002, WINDOW_MS, LINES_PER_WINDOW));
}

#[test]
fn the_next_window_reports_how_many_lines_were_refused() {
    let mut budget = LineBudget::new();
    for _ in 0..LINES_PER_WINDOW {
        budget.admit(1_000, WINDOW_MS, LINES_PER_WINDOW);
    }
    for _ in 0..3 {
        budget.admit(2_000, WINDOW_MS, LINES_PER_WINDOW);
    }
    let next = 1_000 + WINDOW_MS;
    assert!(budget.admit(next, WINDOW_MS, LINES_PER_WINDOW));
    assert_eq!(budget.take_refused(next, WINDOW_MS), 3);
    assert_eq!(budget.take_refused(next, WINDOW_MS), 0);
}

#[test]
fn a_gap_fires_first_then_once_per_gap() {
    let mut budget = LineBudget::new();
    assert!(budget.due(100, 5000));
    assert!(!budget.due(4000, 5000));
    assert!(budget.due(5100, 5000));
    budget.reset();
    assert!(budget.due(5200, 5000));
}

#[test]
fn a_gap_counts_from_the_last_line_that_went_out() {
    let mut budget = LineBudget::new();
    assert!(budget.due(5000, 1000));
    assert!(!budget.due(5999, 1000));
    assert!(budget.due(6000, 1000));
}

#[test]
fn each_key_has_its_own_gap() {
    let mut budgets = KeyedLineBudget::default();
    assert!(budgets.due(3_u16, 10_000, 1000));
    assert!(!budgets.due(3, 10_500, 1000));
    assert!(budgets.due(3, 11_000, 1000));
    assert!(budgets.due(4, 10_500, 1000));
}

#[test]
fn a_forgotten_key_fires_again_at_once() {
    let mut budgets = KeyedLineBudget::default();
    assert!(budgets.due(3_u16, 10_000, 1000));
    budgets.forget(&3);
    assert!(budgets.due(3, 10_100, 1000));
    assert!(!budgets.due(3, 10_200, 1000));
    budgets.clear();
    assert!(budgets.due(3, 10_300, 1000));
}
