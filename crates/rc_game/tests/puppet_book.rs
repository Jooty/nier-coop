use rc_game::offsets::PUPPET_FALLBACK_CLASS_ID;
use rc_game::puppet::{
    BOOK_BOB_PERIOD_MS, BOOK_HOVER_AMPLITUDE, BOOK_PACK_REREQUEST_TIMEOUT_MS,
    BOOK_SPAWN_FIRST_RETRY_MS, BOOK_SPAWN_LOG_EVERY_ATTEMPTS, BOOK_SPAWN_LONGEST_RETRY_MS,
    BookSpawnPlanner, BookSpawnStep, book_class_id, book_hover_offset, book_pack_name,
};
use rc_proto::PartnerBody;

#[test]
fn the_prologue_body_gets_its_own_book_and_the_others_the_default() {
    assert_eq!(book_pack_name(PartnerBody::PrologueCharacter), "bookb012");
    for body in [
        PartnerBody::Nier,
        PartnerBody::SeedNierY,
        PartnerBody::SeedNierF,
        PartnerBody::SeedKaineE,
    ] {
        assert_eq!(book_pack_name(body), "bookw011");
    }
}

#[test]
fn the_book_bobs_around_head_height_and_repeats_each_period() {
    let heights: Vec<f32> = (0..BOOK_BOB_PERIOD_MS)
        .step_by(50)
        .map(|time| book_hover_offset(time)[1])
        .collect();
    let (lowest, highest) = heights.iter().fold((f32::MAX, f32::MIN), |(low, high), h| {
        (low.min(*h), high.max(*h))
    });
    assert!(highest - lowest > BOOK_HOVER_AMPLITUDE);
    assert!(highest - lowest <= 2.0 * BOOK_HOVER_AMPLITUDE + 1e-4);
    assert_eq!(
        book_hover_offset(70),
        book_hover_offset(70 + BOOK_BOB_PERIOD_MS)
    );
}

#[test]
fn the_book_floats_beside_and_behind_the_body() {
    let [side, up, forward] = book_hover_offset(0);
    assert!(side > 0.0);
    assert!(up > 1.0);
    assert!(forward < 0.0);
}

#[test]
fn the_book_uses_the_local_books_class_and_the_fallback_only_without_one() {
    assert_eq!(book_class_id(Some(0x41)), 0x41);
    assert_eq!(book_class_id(None), PUPPET_FALLBACK_CLASS_ID);
    assert_eq!(book_class_id(Some(-1)), PUPPET_FALLBACK_CLASS_ID);
    assert_eq!(book_class_id(Some(0)), PUPPET_FALLBACK_CLASS_ID);
}

#[test]
fn a_missing_pack_is_requested_once_and_again_only_after_the_timeout() {
    let mut planner = BookSpawnPlanner::default();
    assert_eq!(
        planner.next(false, false, 0),
        BookSpawnStep::RequestPack { rerequest: false }
    );
    assert_eq!(planner.next(false, false, 100), BookSpawnStep::Wait);
    assert_eq!(
        planner.next(false, false, BOOK_PACK_REREQUEST_TIMEOUT_MS),
        BookSpawnStep::RequestPack { rerequest: true }
    );
}

#[test]
fn nothing_spawns_or_is_requested_while_loading() {
    let mut planner = BookSpawnPlanner::default();
    assert_eq!(planner.next(true, true, 0), BookSpawnStep::Wait);
    assert_eq!(planner.next(false, true, 10), BookSpawnStep::Wait);
    assert_eq!(
        planner.next(false, false, 20),
        BookSpawnStep::RequestPack { rerequest: false }
    );
}

#[test]
fn failed_spawns_back_off_and_log_the_first_and_every_nth_attempt() {
    let mut planner = BookSpawnPlanner::default();
    assert_eq!(
        planner.next(true, false, 0),
        BookSpawnStep::TrySpawn { attempt: 1, log: true }
    );
    assert_eq!(
        planner.next(true, false, BOOK_SPAWN_FIRST_RETRY_MS - 1),
        BookSpawnStep::Wait
    );
    let mut now = BOOK_SPAWN_FIRST_RETRY_MS;
    assert_eq!(
        planner.next(true, false, now),
        BookSpawnStep::TrySpawn { attempt: 2, log: false }
    );
    let mut logged = Vec::new();
    for _ in 0..BOOK_SPAWN_LOG_EVERY_ATTEMPTS {
        now += BOOK_SPAWN_LONGEST_RETRY_MS;
        if let BookSpawnStep::TrySpawn { attempt, log: true } = planner.next(true, false, now) {
            logged.push(attempt);
        }
    }
    assert_eq!(logged, vec![BOOK_SPAWN_LOG_EVERY_ATTEMPTS]);
    assert_eq!(
        planner.next(true, false, now + BOOK_SPAWN_LONGEST_RETRY_MS - 1),
        BookSpawnStep::Wait
    );
}

#[test]
fn a_shown_book_resets_the_planner() {
    let mut planner = BookSpawnPlanner::default();
    planner.next(true, false, 0);
    planner.spawned();
    assert_eq!(
        planner.next(true, false, 1),
        BookSpawnStep::TrySpawn { attempt: 1, log: true }
    );
}
