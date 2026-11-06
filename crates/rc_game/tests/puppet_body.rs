use rc_game::puppet::{
    BODY_RESEND_INTERVAL_MS, BODY_TABLE, BodyAnnouncer, BodyPackLoad, BodyPackStep, body_info,
    release_requested_pack_wanted,
};
use rc_game::puppet::{PACK_REQUEST_RETRY_MS, PACK_SETTLE_AFTER_FOUND_MS};
use rc_proto::PartnerBody;

#[test]
fn every_body_has_exactly_one_table_row_with_a_distinct_pack() {
    for body in [PartnerBody::Nier, PartnerBody::PrologueCharacter] {
        let rows = BODY_TABLE.iter().filter(|row| row.body == body).count();
        assert_eq!(rows, 1);
    }
    assert_eq!(body_info(PartnerBody::Nier).pack_name, "NIER013");
    assert_eq!(
        body_info(PartnerBody::PrologueCharacter).pack_name,
        "NIER014"
    );
}

#[test]
fn both_bodies_play_the_same_idle_motion_and_show_the_weapon() {
    for row in BODY_TABLE {
        assert_eq!(row.idle_motion, "NIER013_idle_lp");
        assert!(row.weapon_shown);
        assert_eq!((row.request_kind, row.request_slot_hint), (0, 1));
    }
}

#[test]
fn a_resident_pack_is_spawned_at_once_and_never_released() {
    let mut load = BodyPackLoad::new();
    assert_eq!(load.step(true, 0), BodyPackStep::Spawn);
    assert!(!load.requested_by_us());
}

#[test]
fn a_missing_pack_is_requested_once_then_awaited_then_spawned_after_it_settles() {
    let mut load = BodyPackLoad::new();
    assert_eq!(load.step(false, 0), BodyPackStep::Request);
    assert_eq!(load.step(false, 500), BodyPackStep::Wait);
    assert_eq!(load.step(true, 900), BodyPackStep::Wait);
    assert_eq!(
        load.step(true, 900 + PACK_SETTLE_AFTER_FOUND_MS),
        BodyPackStep::Spawn
    );
    assert!(load.requested_by_us());
}

#[test]
fn a_pack_that_never_shows_up_is_requested_again_after_the_retry_time() {
    let mut load = BodyPackLoad::new();
    assert_eq!(load.step(false, 0), BodyPackStep::Request);
    assert_eq!(
        load.step(false, PACK_REQUEST_RETRY_MS),
        BodyPackStep::Request
    );
}

#[test]
fn only_a_pack_we_requested_and_the_local_player_does_not_wear_is_released() {
    assert!(release_requested_pack_wanted(true, false));
    assert!(!release_requested_pack_wanted(true, true));
    assert!(!release_requested_pack_wanted(false, false));
}

#[test]
fn the_body_is_announced_on_change_and_then_once_per_interval() {
    let mut announcer = BodyAnnouncer::new();
    assert!(announcer.is_due(PartnerBody::Nier, 0));
    assert!(!announcer.is_due(PartnerBody::Nier, 10));
    assert!(announcer.is_due(PartnerBody::PrologueCharacter, 20));
    assert!(!announcer.is_due(PartnerBody::PrologueCharacter, 30));
    assert!(announcer.is_due(PartnerBody::PrologueCharacter, 20 + BODY_RESEND_INTERVAL_MS));
}
