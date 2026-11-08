use rc_game::fight_doors::{
    FIGHT_DOOR_PARTNER_RADIUS_UNITS, FIGHT_DOORS, HeldLocks, is_fight_door, judge_fight_door_press,
};
use rc_game::talk_guard::{PressAsk, PressVerdict};
use rc_game::zone_rules::{Companion, Placement, ZoneRadius};

const SHRINE: &str = "A_STONE_TEMPLE_01";
const FIGHT_DOOR: i32 = 608;
const PLAIN_DOOR: i32 = 601;
const PHASE: u16 = 3;

fn partner_at(x: f32) -> Companion {
    Companion {
        name: "Sam".to_owned(),
        placement: Some(Placement {
            phase: PHASE,
            position: [x, 0.0, 0.0],
        }),
        age_ms: 0,
    }
}

fn verdict(door_id: i32, partner_x: f32) -> PressVerdict {
    let partner = partner_at(partner_x);
    judge_fight_door_press(
        SHRINE,
        door_id,
        &PressAsk {
            connected: true,
            partner_game_open: true,
            local: Some(Placement {
                phase: PHASE,
                position: [0.0, 0.0, 0.0],
            }),
            partner: Some(&partner),
            radius: ZoneRadius::default(),
        },
    )
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
fn a_fight_door_waits_for_a_partner_outside_the_small_radius() {
    let outside = FIGHT_DOOR_PARTNER_RADIUS_UNITS + 1.0;
    assert!(matches!(
        verdict(FIGHT_DOOR, outside),
        PressVerdict::Wait { .. }
    ));
}

#[test]
fn a_fight_door_opens_for_a_partner_inside_the_small_radius() {
    let inside = FIGHT_DOOR_PARTNER_RADIUS_UNITS - 1.0;
    assert_eq!(verdict(FIGHT_DOOR, inside), PressVerdict::Allow);
}

#[test]
fn a_partner_inside_the_house_radius_is_still_too_far_for_a_fight_door() {
    let inside_the_default_radius = ZoneRadius::default().units() - 1.0;
    assert!(matches!(
        verdict(FIGHT_DOOR, inside_the_default_radius),
        PressVerdict::Wait { .. }
    ));
}

#[test]
fn a_plain_door_never_waits() {
    assert_eq!(verdict(PLAIN_DOOR, 500.0), PressVerdict::Allow);
}

#[test]
fn a_lock_is_held_only_while_the_guest_is_at_the_door() {
    assert!(HeldLocks::holds_a_lock_near(Some(1.0)));
    assert!(HeldLocks::holds_a_lock_near(Some(
        FIGHT_DOOR_PARTNER_RADIUS_UNITS
    )));
    assert!(!HeldLocks::holds_a_lock_near(Some(
        FIGHT_DOOR_PARTNER_RADIUS_UNITS + 0.5
    )));
    assert!(!HeldLocks::holds_a_lock_near(None));
}

#[test]
fn a_held_lock_is_applied_once_the_guest_leaves_the_door() {
    let mut held = HeldLocks::default();
    held.hold(608);
    held.hold(609);
    let still_there = held.release_those_left_behind(|_| Some(1.0));
    assert!(still_there.is_empty());
    assert_eq!(held.held_ids(), vec![608, 609]);
    let left = held.release_those_left_behind(|id| Some(if id == 608 { 50.0 } else { 1.0 }));
    assert_eq!(left, vec![608]);
    assert!(!held.is_holding(608));
    assert!(held.is_holding(609));
}

#[test]
fn a_held_lock_whose_door_vanished_is_applied_and_forgotten() {
    let mut held = HeldLocks::default();
    held.hold(608);
    assert_eq!(held.release_those_left_behind(|_| None), vec![608]);
    assert!(held.held_ids().is_empty());
}

#[test]
fn an_unlock_drops_the_hold() {
    let mut held = HeldLocks::default();
    held.hold(608);
    assert!(held.drop_hold(608));
    assert!(!held.drop_hold(608));
}
