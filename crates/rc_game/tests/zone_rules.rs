use rc_game::PhaseEntry;
use rc_game::stay_together::{FollowRole, PARTNER_TIMEOUT_MS};
use rc_game::zone_rules::{
    AttemptCooldown, Companion, DEFAULT_ZONE_RADIUS_UNITS, DoorKind, EXIT_DOORS, GUEST_VETO_TEXT,
    HOST_WANTS_TO_GO_COOLDOWN_MS, MAX_ZONE_RADIUS_UNITS, MIN_ZONE_RADIUS_UNITS, Placement,
    VETO_LOG_INTERVAL_MS, VetoNotice, ZoneChangeAsk, ZoneChangeSource, ZoneRadius, ZoneVerdict,
    is_zone_exit_callback, judge_zone_change,
};

const HOME: Placement = Placement {
    phase: 3,
    position: [0.0, 0.0, 0.0],
};

fn companion_at(x: f32, phase: u16, age_ms: u64) -> Companion {
    Companion {
        name: "Sam".to_owned(),
        placement: Some(Placement {
            phase,
            position: [x, 0.0, 0.0],
        }),
        age_ms,
    }
}

fn verdict(role: FollowRole, connected: bool, companions: &[Companion]) -> ZoneVerdict {
    judge_zone_change(&ZoneChangeAsk {
        source: ZoneChangeSource::PlayerChosen,
        role,
        connected,
        partner_game_open: true,
        local: Some(HOME),
        companions,
        radius: ZoneRadius::default(),
    })
}

#[test]
fn a_guest_never_changes_zones() {
    let near = [companion_at(1.0, 3, 0)];
    assert_eq!(
        verdict(FollowRole::Guest, true, &near),
        ZoneVerdict::OnlyHostMayChange
    );
    assert_eq!(
        ZoneVerdict::OnlyHostMayChange.prompt_text().as_deref(),
        Some(GUEST_VETO_TEXT)
    );
}

#[test]
fn nobody_is_stopped_while_playing_alone() {
    assert_eq!(verdict(FollowRole::Guest, false, &[]), ZoneVerdict::Allow);
    assert_eq!(verdict(FollowRole::Host, false, &[]), ZoneVerdict::Allow);
    assert_eq!(verdict(FollowRole::Unknown, true, &[]), ZoneVerdict::Allow);
}

#[test]
fn the_host_changes_zones_when_everyone_is_close_and_in_the_same_phase() {
    let near = [companion_at(DEFAULT_ZONE_RADIUS_UNITS, 3, 100)];
    assert_eq!(verdict(FollowRole::Host, true, &near), ZoneVerdict::Allow);
}

#[test]
fn the_host_waits_for_a_player_beyond_the_radius() {
    let far = [companion_at(DEFAULT_ZONE_RADIUS_UNITS + 0.5, 3, 100)];
    let result = verdict(FollowRole::Host, true, &far);
    assert_eq!(result, ZoneVerdict::WaitingFor("Sam".to_owned()));
    assert_eq!(
        result.prompt_text().as_deref(),
        Some("All players have to be there to change zones (Sam is missing)")
    );
}

#[test]
fn the_host_waits_for_a_player_in_another_phase_even_when_close() {
    let elsewhere = [companion_at(1.0, 4, 100)];
    assert_eq!(
        verdict(FollowRole::Host, true, &elsewhere),
        ZoneVerdict::WaitingFor("Sam".to_owned())
    );
}

#[test]
fn the_host_waits_for_a_player_whose_state_is_stale_or_missing() {
    let stale = [companion_at(1.0, 3, PARTNER_TIMEOUT_MS + 1)];
    assert_eq!(
        verdict(FollowRole::Host, true, &stale),
        ZoneVerdict::WaitingFor("Sam".to_owned())
    );
    let unknown = [Companion {
        name: "Sam".to_owned(),
        placement: None,
        age_ms: 0,
    }];
    assert_eq!(
        verdict(FollowRole::Host, true, &unknown),
        ZoneVerdict::WaitingFor("Sam".to_owned())
    );
}

#[test]
fn the_host_names_the_first_player_who_is_too_far() {
    let mut second = companion_at(500.0, 3, 0);
    "Alex".clone_into(&mut second.name);
    let both = [companion_at(1.0, 3, 0), second];
    assert_eq!(
        verdict(FollowRole::Host, true, &both),
        ZoneVerdict::WaitingFor("Alex".to_owned())
    );
}

#[test]
fn a_larger_radius_lets_the_host_go_with_a_farther_player() {
    let far = [companion_at(40.0, 3, 0)];
    let result = judge_zone_change(&ZoneChangeAsk {
        source: ZoneChangeSource::PlayerChosen,
        role: FollowRole::Host,
        connected: true,
        partner_game_open: true,
        local: Some(HOME),
        companions: &far,
        radius: ZoneRadius::new(50.0),
    });
    assert_eq!(result, ZoneVerdict::Allow);
}

#[test]
fn the_radius_is_clamped_and_ignores_garbage() {
    assert_eq!(ZoneRadius::new(0.0).units(), MIN_ZONE_RADIUS_UNITS);
    assert_eq!(ZoneRadius::new(9999.0).units(), MAX_ZONE_RADIUS_UNITS);
    assert_eq!(ZoneRadius::new(f32::NAN).units(), DEFAULT_ZONE_RADIUS_UNITS);
}

#[test]
fn exit_callbacks_are_found_by_name_and_camera_zones_are_not() {
    assert!(is_zone_exit_callback("JumpMap01_Message"));
    assert!(is_zone_exit_callback("JUMP_MAP_01"));
    assert!(!is_zone_exit_callback("CB_Fixed_Camera_01"));
    assert!(!is_zone_exit_callback("Talk_Text_01"));
    assert!(!is_zone_exit_callback(""));
}

#[test]
fn the_same_veto_is_logged_once_per_interval_and_a_new_one_at_once() {
    let mut notice = VetoNotice::new();
    assert!(notice.record("pop", "text", 0).should_log);
    assert!(
        !notice
            .record("pop", "text", VETO_LOG_INTERVAL_MS - 1)
            .should_log
    );
    assert!(
        notice
            .record("pop", "text", VETO_LOG_INTERVAL_MS)
            .should_log
    );
    assert!(
        notice
            .record("native", "text", VETO_LOG_INTERVAL_MS + 1)
            .should_log
    );
}

#[test]
fn a_scripted_zone_change_always_goes_through_for_the_host_but_never_for_a_guest() {
    let far = [companion_at(500.0, 3, 0)];
    let ask = |role| ZoneChangeAsk {
        source: ZoneChangeSource::Scripted,
        role,
        connected: true,
        partner_game_open: true,
        local: Some(HOME),
        companions: &far,
        radius: ZoneRadius::default(),
    };
    assert_eq!(
        judge_zone_change(&ask(FollowRole::Host)),
        ZoneVerdict::Allow
    );
    assert_eq!(
        judge_zone_change(&ask(FollowRole::Guest)),
        ZoneVerdict::OnlyHostMayChange
    );
}

#[test]
fn an_exit_door_stops_the_guest_and_holds_the_host_until_everyone_is_near() {
    let far = [companion_at(500.0, 3, 0)];
    let near = [companion_at(1.0, 3, 0)];
    let ask = |role, companions| ZoneChangeAsk {
        source: ZoneChangeSource::ExitDoorInteraction,
        role,
        connected: true,
        partner_game_open: true,
        local: Some(HOME),
        companions,
        radius: ZoneRadius::default(),
    };
    assert_eq!(
        judge_zone_change(&ask(FollowRole::Guest, &near)),
        ZoneVerdict::OnlyHostMayChange
    );
    assert_eq!(
        judge_zone_change(&ask(FollowRole::Host, &far)),
        ZoneVerdict::WaitingFor("Sam".to_owned())
    );
    assert_eq!(
        judge_zone_change(&ask(FollowRole::Host, &near)),
        ZoneVerdict::Allow
    );
}

#[test]
fn the_village_library_door_leaves_the_area_and_the_house_and_bar_doors_do_not() {
    assert_eq!(
        DoorKind::of("A_CENTER_VILLAGE_01", 603),
        DoorKind::LeavesTheArea {
            via: "JUMP_MAP_04_01"
        }
    );
    assert_eq!(
        DoorKind::of("A_CENTER_VILLAGE_01", 601),
        DoorKind::StaysInTheArea
    );
    assert_eq!(
        DoorKind::of("A_CENTER_VILLAGE_01", 602),
        DoorKind::StaysInTheArea
    );
}

#[test]
fn a_door_id_means_a_different_door_in_another_phase() {
    assert!(DoorKind::of("A_CENTER_LIBRARY_01", 601).leaves_the_area());
    assert!(!DoorKind::of("A_CENTER_LIBRARY_01", 603).leaves_the_area());
}

#[test]
fn a_phase_whose_every_door_leaves_matches_any_door_id() {
    assert!(DoorKind::of("A_SOUTH_MANSION_02", 1).leaves_the_area());
    assert!(DoorKind::of("A_SOUTH_MANSION_02", -1).leaves_the_area());
}

#[test]
fn unknown_phases_and_doors_stay_in_the_area() {
    assert_eq!(DoorKind::of("", 603), DoorKind::StaysInTheArea);
    assert_eq!(
        DoorKind::of("A_SEASIDE_TOWN_01", 601),
        DoorKind::StaysInTheArea
    );
    assert_eq!(
        DoorKind::of("A_CENTER_VILLAGE_01", -1),
        DoorKind::StaysInTheArea
    );
}

#[test]
fn every_exit_door_names_a_known_phase_once() {
    for (index, door) in EXIT_DOORS.iter().enumerate() {
        assert!(PhaseEntry::find(door.phase).is_some(), "{}", door.phase);
        assert!(!door.via.is_empty());
        assert!(
            EXIT_DOORS[..index]
                .iter()
                .all(|earlier| (earlier.phase, earlier.door_id) != (door.phase, door.door_id)),
            "{} {:?} twice",
            door.phase,
            door.door_id
        );
    }
}

#[test]
fn every_missing_player_is_named() {
    let mut far_one = companion_at(100.0, 3, 0);
    far_one.name = "Ann".to_owned();
    let mut far_two = companion_at(1.0, 9, 0);
    far_two.name = "Bo".to_owned();
    let near = companion_at(1.0, 3, 0);
    assert_eq!(
        verdict(FollowRole::Host, true, &[far_one, near, far_two]),
        ZoneVerdict::WaitingFor("Ann, Bo".to_owned())
    );
}

#[test]
fn the_host_wants_to_go_notice_is_sent_once_per_attempt_then_again_after_the_cooldown() {
    let mut cooldown = AttemptCooldown::new();
    assert!(cooldown.should_send(1_000));
    assert!(!cooldown.should_send(1_016));
    assert!(!cooldown.should_send(1_000 + HOST_WANTS_TO_GO_COOLDOWN_MS - 1));
    assert!(cooldown.should_send(1_000 + HOST_WANTS_TO_GO_COOLDOWN_MS));
}

#[test]
fn the_host_changes_zones_alone_when_the_partner_game_is_closed() {
    let missing = [Companion {
        name: "Sam".to_owned(),
        placement: None,
        age_ms: 0,
    }];
    let ask = |partner_game_open| ZoneChangeAsk {
        source: ZoneChangeSource::PlayerChosen,
        role: FollowRole::Host,
        connected: true,
        partner_game_open,
        local: Some(HOME),
        companions: &missing,
        radius: ZoneRadius::default(),
    };
    assert_eq!(judge_zone_change(&ask(false)), ZoneVerdict::Allow);
    assert_eq!(
        judge_zone_change(&ask(true)),
        ZoneVerdict::WaitingFor("Sam".to_owned())
    );
}

#[test]
fn a_partner_just_inside_the_radius_does_not_open_an_exit_door() {
    let edge = [companion_at(DEFAULT_ZONE_RADIUS_UNITS - 1.0, 3, 0)];
    let ask = |source, companions| ZoneChangeAsk {
        source,
        role: FollowRole::Host,
        connected: true,
        partner_game_open: true,
        local: Some(HOME),
        companions,
        radius: ZoneRadius::default(),
    };
    assert_eq!(
        judge_zone_change(&ask(ZoneChangeSource::ExitDoorInteraction, &edge)),
        ZoneVerdict::WaitingFor("Sam".to_owned())
    );
    assert_eq!(
        judge_zone_change(&ask(ZoneChangeSource::PlayerChosen, &edge)),
        ZoneVerdict::Allow
    );
}

#[test]
fn an_exit_door_opens_when_the_partner_game_is_closed() {
    let far = [companion_at(500.0, 3, 0)];
    let verdict = judge_zone_change(&ZoneChangeAsk {
        source: ZoneChangeSource::ExitDoorInteraction,
        role: FollowRole::Host,
        connected: true,
        partner_game_open: false,
        local: Some(HOME),
        companions: &far,
        radius: ZoneRadius::default(),
    });
    assert_eq!(verdict, ZoneVerdict::Allow);
}
