use replicant_game::one_sided::{
    BOTH_RAN_WINDOW_MS, MAX_PENDING_WATCHED_CALLBACKS, OneSidedTracker, Side, Verdict,
    WATCHED_CALLBACK_TABLE, is_watched_callback, one_sided_log_line,
};
use replicant_game::stay_together::FollowRole;
use replicant_game::story_calls::name_hash;

const CALLBACK: &str = "AttackCount_Talk";
const PHASE: u16 = 7;

#[test]
fn the_watched_table_is_sorted_so_the_lookup_can_search_it() {
    assert!(
        WATCHED_CALLBACK_TABLE
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    );
}

#[test]
fn a_callback_in_the_table_is_watched_only_in_its_own_pack() {
    let (pack, callback) = WATCHED_CALLBACK_TABLE[0];
    assert!(
        WATCHED_CALLBACK_TABLE
            .binary_search(&(pack, callback))
            .is_ok()
    );
    assert!(!is_watched_callback(pack, "NotARealCallbackName"));
    assert!(!is_watched_callback(name_hash("NO_SUCH_PACK"), CALLBACK));
}

#[test]
fn both_games_firing_the_same_callback_in_the_same_phase_is_both_ran() {
    let mut tracker = OneSidedTracker::new();
    assert_eq!(tracker.note(Side::Here, CALLBACK, PHASE, 1_000), None);
    assert_eq!(
        tracker.note(Side::Partner, CALLBACK, PHASE, 4_000),
        Some(Verdict::BothRan {
            name: CALLBACK.to_owned()
        })
    );
    assert_eq!(tracker.waiting(), 0);
    assert!(tracker.expire(100_000).is_empty());
}

#[test]
fn the_order_of_the_two_fires_does_not_matter() {
    let mut tracker = OneSidedTracker::new();
    assert_eq!(tracker.note(Side::Partner, CALLBACK, PHASE, 1_000), None);
    assert!(matches!(
        tracker.note(Side::Here, CALLBACK, PHASE, 2_000),
        Some(Verdict::BothRan { .. })
    ));
}

#[test]
fn a_fire_the_partner_never_matches_is_one_sided_after_ten_seconds() {
    let mut tracker = OneSidedTracker::new();
    tracker.note(Side::Here, CALLBACK, PHASE, 1_000);
    assert!(tracker.expire(1_000 + BOTH_RAN_WINDOW_MS).is_empty());
    assert_eq!(
        tracker.expire(1_001 + BOTH_RAN_WINDOW_MS),
        vec![Verdict::OneSided {
            name: CALLBACK.to_owned(),
            phase: PHASE,
            ran_on: Side::Here
        }]
    );
    assert_eq!(tracker.waiting(), 0);
}

#[test]
fn a_partner_fire_after_the_window_does_not_pair_with_the_old_one() {
    let mut tracker = OneSidedTracker::new();
    tracker.note(Side::Here, CALLBACK, PHASE, 1_000);
    assert_eq!(tracker.note(Side::Partner, CALLBACK, PHASE, 20_000), None);
    assert_eq!(tracker.expire(20_000).len(), 1);
}

#[test]
fn another_phase_or_another_name_does_not_pair() {
    let mut tracker = OneSidedTracker::new();
    tracker.note(Side::Here, CALLBACK, PHASE, 1_000);
    assert_eq!(
        tracker.note(Side::Partner, CALLBACK, PHASE + 1, 1_100),
        None
    );
    assert_eq!(tracker.note(Side::Partner, "Other", PHASE, 1_200), None);
    assert_eq!(tracker.waiting(), 3);
}

#[test]
fn repeated_fires_pair_one_for_one() {
    let mut tracker = OneSidedTracker::new();
    tracker.note(Side::Here, CALLBACK, PHASE, 1_000);
    tracker.note(Side::Here, CALLBACK, PHASE, 1_100);
    assert!(
        tracker
            .note(Side::Partner, CALLBACK, PHASE, 1_200)
            .is_some()
    );
    assert_eq!(tracker.waiting(), 1);
    assert_eq!(tracker.expire(50_000).len(), 1);
}

#[test]
fn the_tracker_holds_a_bounded_number_and_clears() {
    let mut tracker = OneSidedTracker::new();
    for index in 0..MAX_PENDING_WATCHED_CALLBACKS + 10 {
        tracker.note(Side::Here, &format!("cb{index}"), PHASE, 1_000);
    }
    assert_eq!(tracker.waiting(), MAX_PENDING_WATCHED_CALLBACKS);
    tracker.clear();
    assert_eq!(tracker.waiting(), 0);
}

#[test]
fn the_log_line_names_the_game_that_ran_it() {
    let one_sided = |ran_on| Verdict::OneSided {
        name: CALLBACK.to_owned(),
        phase: PHASE,
        ran_on,
    };
    assert_eq!(
        one_sided_log_line(&one_sided(Side::Here), FollowRole::Host, "A_STONE_FIELD_02"),
        "one-sided: AttackCount_Talk ran on host only (A_STONE_FIELD_02, 10 s)"
    );
    assert_eq!(
        one_sided_log_line(&one_sided(Side::Partner), FollowRole::Host, "Z"),
        "one-sided: AttackCount_Talk ran on guest only (Z, 10 s)"
    );
    assert_eq!(
        one_sided_log_line(&one_sided(Side::Partner), FollowRole::Guest, "Z"),
        "one-sided: AttackCount_Talk ran on host only (Z, 10 s)"
    );
    let both = Verdict::BothRan {
        name: CALLBACK.to_owned(),
    };
    assert_eq!(
        one_sided_log_line(&both, FollowRole::Guest, "Z"),
        "both ran: AttackCount_Talk"
    );
}
