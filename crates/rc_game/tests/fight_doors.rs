use rc_game::PHASE_TABLE;
use rc_game::fight_doors::{
    FIGHT_DOORS, FightLockVerdict, HeldLocks, HoldReason, INJECTED_RUN_LOCK_WINDOW_MS, LockOrigin,
    fight_door_in_zone, is_fight_door, judge_door_lock,
};

const SHRINE: &str = "A_STONE_TEMPLE_01";
const FIGHT_DOOR: i32 = 608;
const PLAIN_DOOR: i32 = 601;

fn shrine_index() -> u16 {
    PHASE_TABLE
        .iter()
        .position(|entry| entry.name == SHRINE)
        .and_then(|index| u16::try_from(index).ok())
        .expect("the shrine is in the phase table")
}

#[test]
fn the_table_is_sorted_and_free_of_duplicates() {
    assert!(FIGHT_DOORS.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn the_shrine_fight_rooms_are_listed_and_its_plain_doors_are_not() {
    assert!(is_fight_door(SHRINE, FIGHT_DOOR));
    assert!(is_fight_door(SHRINE, 609));
    assert!(!is_fight_door(SHRINE, PLAIN_DOOR));
    assert!(!is_fight_door("A_CENTER_VILLAGE_01", FIGHT_DOOR));
}

#[test]
fn a_zone_index_finds_its_fight_doors() {
    assert!(fight_door_in_zone(shrine_index(), 608));
    assert!(!fight_door_in_zone(shrine_index(), 601));
    assert!(!fight_door_in_zone(u16::MAX, 608));
}

#[test]
fn a_fight_lock_from_the_players_own_run_applies() {
    assert_eq!(
        judge_door_lock(SHRINE, FIGHT_DOOR, true, LockOrigin::OwnRun),
        FightLockVerdict::Apply
    );
}

#[test]
fn a_fight_lock_from_an_injected_run_is_held() {
    assert_eq!(
        judge_door_lock(SHRINE, FIGHT_DOOR, true, LockOrigin::InjectedRun),
        FightLockVerdict::Hold(HoldReason::InjectedRun)
    );
}

#[test]
fn a_fight_lock_mirrored_from_the_host_is_held() {
    assert_eq!(
        judge_door_lock(SHRINE, FIGHT_DOOR, true, LockOrigin::MirroredFromHost),
        FightLockVerdict::Hold(HoldReason::MirroredFromHost)
    );
}

#[test]
fn an_unlock_always_applies() {
    for origin in [
        LockOrigin::OwnRun,
        LockOrigin::InjectedRun,
        LockOrigin::MirroredFromHost,
    ] {
        assert_eq!(
            judge_door_lock(SHRINE, FIGHT_DOOR, false, origin),
            FightLockVerdict::Apply
        );
    }
}

#[test]
fn a_plain_door_lock_always_applies() {
    for origin in [
        LockOrigin::OwnRun,
        LockOrigin::InjectedRun,
        LockOrigin::MirroredFromHost,
    ] {
        assert_eq!(
            judge_door_lock(SHRINE, PLAIN_DOOR, true, origin),
            FightLockVerdict::Apply
        );
    }
}

#[test]
fn a_lock_right_after_an_injected_run_started_is_injected() {
    assert_eq!(
        LockOrigin::of_a_local_lock(Some(0)),
        LockOrigin::InjectedRun
    );
    assert_eq!(
        LockOrigin::of_a_local_lock(Some(INJECTED_RUN_LOCK_WINDOW_MS - 1)),
        LockOrigin::InjectedRun
    );
}

#[test]
fn a_lock_long_after_an_injected_run_or_with_none_is_the_players_own() {
    assert_eq!(
        LockOrigin::of_a_local_lock(Some(INJECTED_RUN_LOCK_WINDOW_MS)),
        LockOrigin::OwnRun
    );
    assert_eq!(LockOrigin::of_a_local_lock(None), LockOrigin::OwnRun);
}

#[test]
fn an_unlock_drops_the_hold() {
    let mut held = HeldLocks::default();
    held.hold(608);
    assert!(held.is_holding(608));
    assert!(held.drop_hold(608));
    assert!(!held.drop_hold(608));
    assert!(!held.is_holding(608));
}

#[test]
fn clearing_forgets_every_hold() {
    let mut held = HeldLocks::default();
    held.hold(608);
    held.hold(609);
    held.clear();
    assert!(!held.is_holding(608) && !held.is_holding(609));
}
