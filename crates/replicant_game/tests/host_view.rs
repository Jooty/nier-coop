use replicant_game::scene_follow::{
    GUEST_TAKES_HOST_VIEW_WITHIN_UNITS, HOST_VIEW_RELEASED_AFTER_GUEST_FAR_MS, HostViewFacts,
    HostViewHold, HostViewKind, HostViewRelease, HostViewVerdict, guest_to_host_units,
};
use replicant_game::story_calls::Whereabouts;

const PHASE: u16 = 7;
const RUN: &str = "EID_1410_a0065g_015_Start";

fn at(position: [f32; 3]) -> Option<Whereabouts> {
    Some(Whereabouts {
        phase: PHASE,
        position,
    })
}

fn hosts_call_on_guest(units: Option<f32>) -> HostViewFacts {
    HostViewFacts {
        is_guest: true,
        called_by_the_mod: false,
        in_fight_window: false,
        in_event_scene: false,
        host_led_run: true,
        gives_the_view_back: false,
        guest_to_host_units: units,
    }
}

#[test]
fn the_runs_long_stretch_apart_measures_61_units() {
    let units = guest_to_host_units(at([0.1, 12.0, 2.6]), at([60.7, 20.0, 6.1]));
    assert!(units.is_some_and(|units| (units - 61.3).abs() < 0.1));
}

#[test]
fn other_phases_have_no_distance() {
    let host = Some(Whereabouts {
        phase: PHASE + 1,
        position: [0.0; 3],
    });
    assert_eq!(guest_to_host_units(at([0.0; 3]), host), None);
}

#[test]
fn a_hosts_camera_reaches_a_guest_standing_near() {
    for units in [0.0, 7.2, 9.0, 14.5, 16.8] {
        assert_eq!(
            HostViewVerdict::judge(&hosts_call_on_guest(Some(units))),
            HostViewVerdict::Apply { units }
        );
    }
}

#[test]
fn a_hosts_camera_is_left_out_for_a_guest_far_away() {
    for units in [21.6, 61.3, 80.4, 90.8] {
        let verdict = HostViewVerdict::judge(&hosts_call_on_guest(Some(units)));
        assert_eq!(verdict, HostViewVerdict::Leave { units: Some(units) });
        assert!(!verdict.runs());
    }
    assert_eq!(
        HostViewVerdict::judge(&hosts_call_on_guest(None)),
        HostViewVerdict::Leave { units: None }
    );
}

#[test]
fn the_reach_is_inclusive() {
    let verdict = HostViewVerdict::judge(&hosts_call_on_guest(Some(
        GUEST_TAKES_HOST_VIEW_WITHIN_UNITS,
    )));
    assert!(verdict.runs());
}

#[test]
fn a_fight_window_keeps_the_hosts_fixed_camera_on_a_far_guest() {
    let facts = HostViewFacts {
        in_fight_window: true,
        ..hosts_call_on_guest(Some(61.3))
    };
    assert_eq!(HostViewVerdict::judge(&facts), HostViewVerdict::NotGated);
}

#[test]
fn an_event_scene_takes_the_guest_wherever_it_stands() {
    let facts = HostViewFacts {
        in_event_scene: true,
        ..hosts_call_on_guest(Some(90.8))
    };
    assert_eq!(HostViewVerdict::judge(&facts), HostViewVerdict::NotGated);
}

#[test]
fn the_guests_own_scripts_and_the_mods_calls_are_not_gated() {
    let own = HostViewFacts {
        host_led_run: false,
        ..hosts_call_on_guest(Some(61.3))
    };
    let modded = HostViewFacts {
        called_by_the_mod: true,
        ..hosts_call_on_guest(Some(61.3))
    };
    let host = HostViewFacts {
        is_guest: false,
        ..hosts_call_on_guest(Some(61.3))
    };
    for facts in [own, modded, host] {
        assert_eq!(HostViewVerdict::judge(&facts), HostViewVerdict::NotGated);
    }
}

#[test]
fn a_far_guest_still_gets_its_view_back() {
    let facts = HostViewFacts {
        gives_the_view_back: true,
        ..hosts_call_on_guest(Some(61.3))
    };
    assert!(HostViewVerdict::judge(&facts).runs());
}

#[test]
fn a_held_camera_is_released_once_the_guest_stays_far() {
    let mut hold = HostViewHold::new();
    hold.host_camera_applied(RUN, false);
    assert!(hold.step(Some(9.0), 0).is_empty());
    assert!(hold.step(Some(21.1), 1_000).is_empty());
    assert!(
        hold.step(
            Some(55.9),
            1_000 + HOST_VIEW_RELEASED_AFTER_GUEST_FAR_MS - 1
        )
        .is_empty()
    );
    assert_eq!(
        hold.step(Some(75.8), 1_000 + HOST_VIEW_RELEASED_AFTER_GUEST_FAR_MS),
        vec![HostViewRelease {
            kind: HostViewKind::Camera,
            run: RUN.to_owned(),
        }]
    );
    assert!(!hold.holds_any());
    assert!(hold.step(Some(90.6), 10_000).is_empty());
}

#[test]
fn coming_back_near_restarts_the_wait() {
    let mut hold = HostViewHold::new();
    hold.host_fade_applied(RUN, true);
    assert!(hold.step(Some(30.0), 0).is_empty());
    assert!(hold.step(Some(5.0), 500).is_empty());
    assert!(hold.step(Some(30.0), 600).is_empty());
    assert!(
        hold.step(Some(30.0), 600 + HOST_VIEW_RELEASED_AFTER_GUEST_FAR_MS - 1)
            .is_empty()
    );
    let released = hold.step(Some(30.0), 600 + HOST_VIEW_RELEASED_AFTER_GUEST_FAR_MS);
    assert_eq!(released.len(), 1);
    assert_eq!(released[0].kind, HostViewKind::Fade);
}

#[test]
fn the_free_camera_or_the_guests_own_camera_ends_the_hold() {
    let mut hold = HostViewHold::new();
    hold.host_camera_applied(RUN, false);
    hold.host_camera_applied(RUN, true);
    assert!(!hold.holds_any());
    hold.host_camera_applied(RUN, false);
    hold.own_camera_call();
    assert!(!hold.holds_any());
    hold.host_fade_applied(RUN, true);
    hold.host_fade_applied(RUN, false);
    assert!(!hold.holds_any());
}
