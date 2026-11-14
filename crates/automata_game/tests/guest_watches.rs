use automata_game::character::Character;
use automata_game::guest_watches::{
    FullLoadVerdict, GuestWatch, HAND_BACK_LOAD_MAY_TAKE_MS, HERO_INPUT_SOURCE_OFF,
    HERO_INPUT_SOURCE_ON, HeroInputSources, StateDifference, WATCHED_HERO_STATE_MAY_DIFFER_FOR_MS,
    WATCHED_PHASE_ID, WatchFeedReadings, WatchFeedRefusal, WatchReadings, WatchRole,
    WatchedStretch, WatchingReading, guest_watch_applies, hero_fields_given_back,
    hero_fields_while_watched, hero_written_for_a_start, host_is_watched, watch_feed_refusal,
    watching_line,
};
use automata_game::load_gate::StoryPlace;
use automata_game::partner::BodyFields;
use automata_game::start::NEW_GAME_OPENING_PHASE_NAME;
use automata_game::travel::JumpRole;

const MADE_UP_WATCHED_NAME: &str = "00_99_Z_RobotM_Pro_MadeUp";
const MADE_UP_UNWATCHED_NAME: &str = "01_00_A_MadeUp_Place";
const MADE_UP_LATER_NAME: &str = "01_40_A_MadeUp_Later";
const OTHER_PHASE_ID: i32 = 0x200;

fn opening() -> StoryPlace {
    StoryPlace::new(WATCHED_PHASE_ID, NEW_GAME_OPENING_PHASE_NAME)
}

fn place(name: &str) -> StoryPlace {
    StoryPlace::new(WATCHED_PHASE_ID, name)
}

fn readings(now_ms: u64, place: StoryPlace, in_play: bool) -> WatchReadings {
    WatchReadings {
        now_ms,
        applies: true,
        place,
        in_play,
        hero: Some(Character::TwoB),
        host: Some(Character::TwoB),
    }
}

fn watching_machine() -> GuestWatch {
    let mut machine = GuestWatch::new(WatchedStretch::default());
    let step = machine.step(&readings(0, opening(), true));
    assert!(step.started && step.watching);
    machine
}

fn hero_fields() -> BodyFields {
    BodyFields {
        flags: 0x40_0000,
        player_controlled: 1,
        pad_enable: 1,
        mouse_enable: 1,
        keyboard_enable: 1,
        input_ignore_seconds: 0.5,
        run_speed_type: 0,
        dismissed: 0,
    }
}

fn free_feed() -> WatchFeedReadings {
    WatchFeedReadings {
        stick_is_hooked: true,
        host_state_is_fresh: true,
        local_phase_id: WATCHED_PHASE_ID,
        host_phase_id: Some(WATCHED_PHASE_ID),
        a_jump_is_pending: false,
        local_free: true,
        host_free: true,
    }
}

#[test]
fn the_new_game_place_and_any_marked_sub_phase_of_the_phase_are_watched() {
    let stretch = WatchedStretch::default();
    assert!(stretch.holds(&opening()));
    assert!(stretch.holds(&place(MADE_UP_WATCHED_NAME)));
}

#[test]
fn an_unmarked_sub_phase_or_another_phase_is_not_watched() {
    let stretch = WatchedStretch::default();
    assert!(!stretch.holds(&place(MADE_UP_UNWATCHED_NAME)));
    assert!(!stretch.holds(&StoryPlace::new(
        OTHER_PHASE_ID,
        NEW_GAME_OPENING_PHASE_NAME
    )));
    assert!(!stretch.holds(&StoryPlace::new(WATCHED_PHASE_ID, "")));
}

#[test]
fn without_the_setting_a_full_load_out_of_the_marked_names_ends_watching() {
    let stretch = WatchedStretch::new(None);
    assert!(stretch.a_full_load_there_ends_watching(&place(MADE_UP_UNWATCHED_NAME)));
    assert!(!stretch.a_full_load_there_ends_watching(&place(MADE_UP_WATCHED_NAME)));
}

#[test]
fn the_end_setting_moves_the_end_to_its_sub_phase_and_any_other_phase() {
    let stretch = WatchedStretch::new(Some(MADE_UP_LATER_NAME));
    assert!(!stretch.a_full_load_there_ends_watching(&place(MADE_UP_UNWATCHED_NAME)));
    assert!(stretch.a_full_load_there_ends_watching(&place(MADE_UP_LATER_NAME)));
    assert!(
        stretch.a_full_load_there_ends_watching(&StoryPlace::new(
            OTHER_PHASE_ID,
            MADE_UP_UNWATCHED_NAME
        ))
    );
}

#[test]
fn a_blank_end_setting_is_no_setting() {
    assert_eq!(WatchedStretch::new(Some("  ")), WatchedStretch::default());
    assert_eq!(WatchedStretch::new(Some("")).ends_at(), None);
}

#[test]
fn the_end_place_itself_is_never_watched() {
    let stretch = WatchedStretch::new(Some(MADE_UP_WATCHED_NAME));
    assert!(!stretch.holds(&place(MADE_UP_WATCHED_NAME)));
    assert!(stretch.holds(&opening()));
}

#[test]
fn only_a_campaign_guest_with_its_host_present_and_the_setting_on_watches() {
    assert!(guest_watch_applies(true, JumpRole::Guest, true));
    assert!(!guest_watch_applies(false, JumpRole::Guest, true));
    assert!(!guest_watch_applies(true, JumpRole::Guest, false));
    assert!(!guest_watch_applies(true, JumpRole::Host, true));
    assert!(!guest_watch_applies(true, JumpRole::Solo, true));
}

#[test]
fn the_host_is_watched_only_with_its_own_setting_on_and_the_guests_word() {
    assert!(host_is_watched(true, JumpRole::Host, true));
    assert!(!host_is_watched(true, JumpRole::Host, false));
    assert!(!host_is_watched(false, JumpRole::Host, true));
    assert!(!host_is_watched(true, JumpRole::Guest, true));
    assert!(!host_is_watched(true, JumpRole::Solo, true));
}

#[test]
fn neither_copy_has_a_partner_body_while_the_guest_watches() {
    assert!(WatchRole::Watches.has_no_partner_body());
    assert!(WatchRole::IsWatched.has_no_partner_body());
    assert!(!WatchRole::Neither.has_no_partner_body());
}

#[test]
fn the_watch_role_survives_its_number() {
    for role in [WatchRole::Neither, WatchRole::Watches, WatchRole::IsWatched] {
        assert_eq!(WatchRole::from_number(role.as_number()), role);
    }
    assert_eq!(WatchRole::from_number(200), WatchRole::Neither);
}

#[test]
fn watching_starts_in_a_watched_place_even_before_play() {
    let mut machine = GuestWatch::new(WatchedStretch::default());
    let step = machine.step(&readings(0, opening(), false));
    assert!(step.started && step.watching);
    assert!(step.logs[0].starts_with("watching: started at"));
}

#[test]
fn watching_does_not_start_where_it_does_not_apply_or_outside_the_stretch() {
    let mut machine = GuestWatch::new(WatchedStretch::default());
    let not_applying = WatchReadings {
        applies: false,
        ..readings(0, opening(), true)
    };
    assert!(!machine.step(&not_applying).watching);
    assert!(
        !machine
            .step(&readings(10, place(MADE_UP_UNWATCHED_NAME), true))
            .watching
    );
    assert!(machine.step(&readings(20, opening(), true)).started);
}

#[test]
fn leaving_the_stretch_without_a_full_load_keeps_watching_with_a_line_a_second() {
    let mut machine = watching_machine();
    let outside = place(MADE_UP_UNWATCHED_NAME);
    let first = machine.step(&readings(100, outside, true));
    assert!(first.watching);
    assert_eq!(first.logs.len(), 1);
    assert!(first.logs[0].contains("watching goes on until the next full load"));
    assert!(machine.step(&readings(600, outside, true)).logs.is_empty());
    assert_eq!(machine.step(&readings(1_100, outside, true)).logs.len(), 1);
}

#[test]
fn a_full_load_inside_the_stretch_keeps_watching() {
    let mut machine = watching_machine();
    assert_eq!(
        machine.before_a_full_load(10, &place(MADE_UP_WATCHED_NAME)),
        FullLoadVerdict::StaysWatching
    );
    assert!(machine.step(&readings(20, opening(), false)).watching);
}

#[test]
fn a_full_load_out_of_the_stretch_hands_back_and_watching_ends_when_the_load_ends() {
    let mut machine = watching_machine();
    let target = place(MADE_UP_UNWATCHED_NAME);
    assert_eq!(
        machine.before_a_full_load(10, &target),
        FullLoadVerdict::HandsBack
    );
    let still_in_the_old_place = machine.step(&readings(20, opening(), true));
    assert!(still_in_the_old_place.watching && !still_in_the_old_place.ended);
    assert!(machine.step(&readings(30, opening(), false)).watching);
    let arrived = machine.step(&readings(40, target, true));
    assert!(arrived.ended && !arrived.watching);
    assert!(arrived.logs[0].starts_with("watching: ended, the full load ended at"));
    assert!(machine.step(&readings(50, target, true)).logs.is_empty());
}

#[test]
fn a_hand_back_whose_load_never_ends_gives_up_after_the_bound() {
    let mut machine = watching_machine();
    machine.before_a_full_load(0, &place(MADE_UP_UNWATCHED_NAME));
    assert!(
        machine
            .step(&readings(HAND_BACK_LOAD_MAY_TAKE_MS, opening(), true))
            .watching
    );
    let given_up = machine.step(&readings(HAND_BACK_LOAD_MAY_TAKE_MS + 1, opening(), true));
    assert!(given_up.ended && !given_up.watching);
    assert!(given_up.logs[0].contains("did not end within"));
}

#[test]
fn a_full_load_while_not_watching_is_left_alone() {
    let mut machine = GuestWatch::new(WatchedStretch::default());
    assert_eq!(
        machine.before_a_full_load(0, &place(MADE_UP_UNWATCHED_NAME)),
        FullLoadVerdict::NotWatching
    );
}

#[test]
fn watching_ends_at_once_when_the_host_is_gone() {
    let mut machine = watching_machine();
    let gone = WatchReadings {
        applies: false,
        ..readings(10, opening(), true)
    };
    let step = machine.step(&gone);
    assert!(step.ended && !step.watching);
}

#[test]
fn the_start_writes_the_hosts_character_only_into_a_watched_place() {
    let stretch = WatchedStretch::default();
    assert_eq!(
        hero_written_for_a_start(true, &stretch, &opening(), Character::TwoB),
        Some(Character::TwoB)
    );
    assert_eq!(
        hero_written_for_a_start(
            true,
            &stretch,
            &place(MADE_UP_UNWATCHED_NAME),
            Character::TwoB
        ),
        None
    );
    assert_eq!(
        hero_written_for_a_start(false, &stretch, &opening(), Character::TwoB),
        None
    );
}

#[test]
fn watching_cuts_only_the_heros_three_input_sources() {
    let current = hero_fields();
    let watched = hero_fields_while_watched(current, false);
    assert_eq!(
        HeroInputSources::of(watched),
        HeroInputSources {
            pad: HERO_INPUT_SOURCE_OFF,
            mouse: HERO_INPUT_SOURCE_OFF,
            keyboard: HERO_INPUT_SOURCE_OFF,
        }
    );
    assert_eq!(watched.player_controlled, current.player_controlled);
    assert_eq!(watched.flags, current.flags);
    assert_eq!(watched.input_ignore_seconds, current.input_ignore_seconds);
    assert_eq!(watched.run_speed_type, current.run_speed_type);
    assert_eq!(watched.dismissed, current.dismissed);
}

#[test]
fn the_pad_source_opens_alone_while_the_host_flies_on_a_fed_stick() {
    let watched = hero_fields_while_watched(hero_fields(), true);
    assert_eq!(watched.pad_enable, HERO_INPUT_SOURCE_ON);
    assert_eq!(watched.mouse_enable, HERO_INPUT_SOURCE_OFF);
    assert_eq!(watched.keyboard_enable, HERO_INPUT_SOURCE_OFF);
}

#[test]
fn the_give_back_writes_the_kept_sources_or_all_on() {
    let cut = hero_fields_while_watched(hero_fields(), false);
    let kept = HeroInputSources {
        pad: 1,
        mouse: 0,
        keyboard: 1,
    };
    assert_eq!(
        HeroInputSources::of(hero_fields_given_back(cut, Some(kept))),
        kept
    );
    assert_eq!(
        HeroInputSources::of(hero_fields_given_back(cut, None)),
        HeroInputSources::ALL_ON
    );
    assert_eq!(hero_fields_given_back(cut, None).player_controlled, 1);
}

#[test]
fn sources_read_all_off_are_not_kept() {
    let all_off = HeroInputSources::of(hero_fields_while_watched(hero_fields(), false));
    assert_eq!(all_off.worth_keeping(), None);
    let some_on = HeroInputSources::of(hero_fields());
    assert_eq!(some_on.worth_keeping(), Some(some_on));
}

#[test]
fn the_hero_is_fed_and_placed_only_when_both_are_free_in_one_phase() {
    assert_eq!(watch_feed_refusal(free_feed()), None);
    let cases = [
        (
            WatchFeedReadings {
                stick_is_hooked: false,
                ..free_feed()
            },
            WatchFeedRefusal::StickNotHooked,
        ),
        (
            WatchFeedReadings {
                host_state_is_fresh: false,
                ..free_feed()
            },
            WatchFeedRefusal::NoFreshHostState,
        ),
        (
            WatchFeedReadings {
                host_phase_id: Some(OTHER_PHASE_ID),
                ..free_feed()
            },
            WatchFeedRefusal::OtherPhase {
                local: WATCHED_PHASE_ID,
                host: OTHER_PHASE_ID,
            },
        ),
        (
            WatchFeedReadings {
                a_jump_is_pending: true,
                ..free_feed()
            },
            WatchFeedRefusal::JumpPending,
        ),
        (
            WatchFeedReadings {
                local_free: false,
                ..free_feed()
            },
            WatchFeedRefusal::LocalNotFree,
        ),
        (
            WatchFeedReadings {
                host_free: false,
                ..free_feed()
            },
            WatchFeedRefusal::HostNotFree,
        ),
    ];
    for (readings, refusal) in cases {
        assert_eq!(watch_feed_refusal(readings), Some(refusal));
    }
}

#[test]
fn a_state_difference_is_logged_once_after_the_bound_and_again_when_it_is_gone() {
    let mut difference = StateDifference::default();
    assert_eq!(difference.observe(0, 0x10, 0x20), None);
    assert_eq!(
        difference.observe(WATCHED_HERO_STATE_MAY_DIFFER_FOR_MS, 0x10, 0x20),
        None
    );
    let line = difference.observe(WATCHED_HERO_STATE_MAY_DIFFER_FOR_MS + 1, 0x10, 0x20);
    assert!(line.is_some_and(|line| line.contains("no state is started on the hero")));
    assert_eq!(difference.observe(5_000, 0x10, 0x20), None);
    assert!(
        difference
            .observe(6_000, 0x20, 0x20)
            .is_some_and(|line| line.contains("again"))
    );
    assert_eq!(difference.observe(7_000, 0x20, 0x20), None);
}

#[test]
fn a_short_state_difference_says_nothing() {
    let mut difference = StateDifference::default();
    assert_eq!(difference.observe(0, 0x10, 0x20), None);
    assert_eq!(difference.observe(100, 0x20, 0x20), None);
}

#[test]
fn the_once_a_second_line_says_what_the_camera_is_on_and_both_places() {
    let line = watching_line(&WatchingReading {
        hero_position: Some([0.0, 0.0, 0.0]),
        host_position: Some([3.0, 0.0, 4.0]),
        hero_state: Some(0x10),
        host_state: Some(0x10),
        hero_health: Some(80),
        host_health: Some((80, 100)),
        hero_rides: false,
        refusal: None,
    });
    assert!(line.contains("the camera is on this game's hero, driven as the host's body"));
    assert!(line.contains("error Some(5.0)"));
    assert!(line.ends_with("fed and placed"));
}
