use rc_game::Position;
use rc_game::view_follow::{
    DoorView, RetryAction, SCENE_TELEPORT_WINDOW_MS, SceneWatch, VIEW_RESEND_MS, VIEW_RETRY_CAP_MS,
    VIEW_RETRY_INTERVAL_MS, VIEW_SCENE_OVER_FALLBACK_MS, VIEW_SCENE_WAIT_CAP_MS, ViewAnnouncer,
    ViewFollow, ViewRetry, decide_view_follow, door_chain_event, view_after_callback,
    view_after_phase_check, view_chain,
};

const HOUSE: &str = "of_NierHouse";
const BAR: &str = "of_CenterBar";

fn view(chain: &str, inside: bool) -> DoorView {
    DoorView {
        chain: chain.to_owned(),
        inside,
    }
}

fn at(x: f32) -> Position {
    Position {
        x,
        y: 14.0,
        z: 10.0,
    }
}

#[test]
fn the_host_inside_and_the_guest_outside_enters_the_house_view() {
    let host = view(HOUSE, true);
    let own = view(HOUSE, false);
    assert_eq!(
        decide_view_follow(Some(&host), Some(&own), true),
        ViewFollow::Enter {
            chain: HOUSE.to_owned()
        }
    );
    assert_eq!(
        decide_view_follow(Some(&host), None, true),
        ViewFollow::Enter {
            chain: HOUSE.to_owned()
        }
    );
}

#[test]
fn the_host_outside_and_the_guest_inside_leaves_the_view() {
    let host = view(HOUSE, false);
    let own = view(HOUSE, true);
    assert_eq!(
        decide_view_follow(Some(&host), Some(&own), true),
        ViewFollow::Leave {
            chain: HOUSE.to_owned()
        }
    );
    assert_eq!(
        decide_view_follow(None, Some(&own), true),
        ViewFollow::Leave {
            chain: HOUSE.to_owned()
        }
    );
}

#[test]
fn the_same_state_changes_nothing() {
    let inside = view(HOUSE, true);
    let outside = view(HOUSE, false);
    assert_eq!(
        decide_view_follow(Some(&inside), Some(&inside), true),
        ViewFollow::Nothing
    );
    assert_eq!(
        decide_view_follow(Some(&outside), Some(&outside), true),
        ViewFollow::Nothing
    );
    assert_eq!(decide_view_follow(None, None, true), ViewFollow::Nothing);
}

#[test]
fn a_guest_inside_another_building_enters_the_hosts_building() {
    let host = view(BAR, true);
    let own = view(HOUSE, true);
    assert_eq!(
        decide_view_follow(Some(&host), Some(&own), true),
        ViewFollow::Enter {
            chain: BAR.to_owned()
        }
    );
}

#[test]
fn no_scene_that_moved_the_player_changes_nothing() {
    let host = view(HOUSE, true);
    assert_eq!(
        decide_view_follow(Some(&host), None, false),
        ViewFollow::Nothing
    );
}

#[test]
fn an_unknown_chain_changes_nothing() {
    let host = view("of_Nowhere", true);
    assert_eq!(
        decide_view_follow(Some(&host), None, true),
        ViewFollow::Nothing
    );
}

#[test]
fn door_callback_names_give_their_chain_and_side() {
    let entered = door_chain_event("GM_IN_Door_of_NierHouse_02").unwrap();
    assert_eq!((entered.chain, entered.inside), (HOUSE, true));
    let left = door_chain_event("GM_OUT_Door_of_NierHouse_14").unwrap();
    assert_eq!((left.chain, left.inside), (HOUSE, false));
    let robot = door_chain_event("GM_IN_Door_RobotHouse03").unwrap();
    assert_eq!((robot.chain, robot.inside), ("RobotHouse", true));
    let second = door_chain_event("GM_OUT_Door_of_PopolRoom2_02").unwrap();
    assert_eq!((second.chain, second.inside), ("of_PopolRoom2", false));
}

#[test]
fn the_walk_to_the_door_and_other_names_are_not_door_events() {
    assert_eq!(door_chain_event("GM_IN_Door_of_NierHouse_01"), None);
    assert_eq!(door_chain_event("GM_IN_Door_of_Slope_01_02"), None);
    assert_eq!(door_chain_event("GM_IN_Door_of_Slope_01_FAST"), None);
    assert_eq!(door_chain_event("CB_SideView_SetBright_NierHome_In"), None);
    assert_eq!(door_chain_event("GM_IN_Door_"), None);
    assert_eq!(door_chain_event("GM_OUT_Door_é"), None);
}

#[test]
fn the_house_entry_runs_the_view_part_then_the_brightness_callback() {
    let chain = view_chain(HOUSE).unwrap();
    assert_eq!(
        chain.entry,
        [
            "GM_IN_Door_of_NierHouse_02",
            "CB_SideView_SetBright_NierHome_In"
        ]
    );
    assert_eq!(
        chain.exit,
        [
            "GM_OUT_Door_of_NierHouse_02",
            "CB_SideView_SetBright_NierHome_Out"
        ]
    );
}

#[test]
fn a_jump_inside_the_window_counts_and_a_walk_does_not() {
    let mut watch = SceneWatch::new();
    assert!(!watch.observe(0, at(0.0)));
    watch.open(100);
    assert!(!watch.observe(200, at(1.0)));
    assert!(watch.observe(300, at(60.0)));
    assert!(!watch.observe(400, at(200.0)));
}

#[test]
fn a_jump_after_the_window_or_with_no_scene_does_not_count() {
    let mut watch = SceneWatch::new();
    assert!(!watch.observe(0, at(0.0)));
    assert!(!watch.observe(10, at(100.0)));
    watch.open(20);
    assert!(!watch.observe(21 + SCENE_TELEPORT_WINDOW_MS, at(300.0)));
    assert!(!watch.observe(22 + SCENE_TELEPORT_WINDOW_MS, at(600.0)));
}

#[test]
fn the_announcer_sends_changes_at_once_and_repeats_later() {
    let mut announcer = ViewAnnouncer::new();
    let outside = view(HOUSE, false);
    let inside = view(HOUSE, true);
    assert!(!announcer.is_due(None, 0));
    assert!(announcer.is_due(Some(&outside), 0));
    assert!(!announcer.is_due(Some(&outside), 100));
    assert!(announcer.is_due(Some(&inside), 200));
    assert!(announcer.is_due(Some(&inside), 200 + VIEW_RESEND_MS));
    announcer.reset();
    assert!(announcer.is_due(Some(&inside), 300));
}

const ENTRY: &str = "GM_IN_Door_of_NierHouse_02";
const BRIGHT: &str = "CB_SideView_SetBright_NierHome_In";
const NATURAL: &str = "EID_0100_a0039g_Start";

fn requested(now: u64) -> ViewRetry {
    let mut retry = ViewRetry::new();
    retry.request(&[ENTRY, BRIGHT], now);
    retry
}

#[test]
fn nothing_is_queued_while_the_scene_is_still_running() {
    let mut retry = requested(1_000);
    assert!(retry.poll(1_500).is_empty());
    retry.note_fade(true);
    retry.note_pop(NATURAL, 2_000);
    assert!(retry.poll(3_000).is_empty());
}

#[test]
fn both_names_are_queued_once_the_fade_finished_and_a_natural_callback_popped() {
    let mut retry = requested(1_000);
    retry.note_fade(true);
    retry.note_fade(false);
    assert!(retry.poll(2_000).is_empty());
    retry.note_pop(NATURAL, 2_100);
    assert_eq!(
        retry.poll(2_200),
        vec![
            RetryAction::Queue(ENTRY.to_owned()),
            RetryAction::Queue(BRIGHT.to_owned())
        ]
    );
}

#[test]
fn without_any_fade_a_natural_callback_and_the_fallback_time_end_the_wait() {
    let mut retry = requested(0);
    retry.note_pop(NATURAL, 100);
    assert!(retry.poll(VIEW_SCENE_OVER_FALLBACK_MS - 1).is_empty());
    assert_eq!(retry.poll(VIEW_SCENE_OVER_FALLBACK_MS).len(), 2);
}

#[test]
fn a_scene_that_never_ends_is_given_up_waiting_for_at_the_wait_cap() {
    let mut retry = requested(0);
    retry.note_fade(true);
    assert!(retry.poll(VIEW_SCENE_WAIT_CAP_MS - 1).is_empty());
    assert_eq!(retry.poll(VIEW_SCENE_WAIT_CAP_MS).len(), 2);
}

#[test]
fn a_queued_name_is_queued_again_every_interval_until_it_pops() {
    let mut retry = requested(0);
    retry.note_pop(NATURAL, 1);
    let start = VIEW_SCENE_OVER_FALLBACK_MS;
    assert_eq!(retry.poll(start).len(), 2);
    assert!(retry.poll(start + VIEW_RETRY_INTERVAL_MS - 1).is_empty());
    assert_eq!(retry.poll(start + VIEW_RETRY_INTERVAL_MS).len(), 2);
    let ran = retry.note_pop(ENTRY, start + 2_500 + VIEW_RETRY_INTERVAL_MS);
    assert_eq!(ran.map(|ran| ran.after_ms), Some(4_500));
    assert_eq!(
        retry.poll(start + 2 * VIEW_RETRY_INTERVAL_MS),
        vec![RetryAction::Queue(BRIGHT.to_owned())]
    );
}

#[test]
fn a_name_that_never_pops_is_reported_and_dropped_at_the_cap() {
    let mut retry = requested(0);
    retry.note_pop(NATURAL, 1);
    let start = VIEW_SCENE_OVER_FALLBACK_MS;
    retry.poll(start);
    retry.note_pop(ENTRY, start + 1);
    let actions = retry.poll(start + VIEW_RETRY_CAP_MS);
    assert_eq!(actions, vec![RetryAction::GiveUp(BRIGHT.to_owned())]);
    assert!(retry.is_idle());
}

#[test]
fn the_inside_state_comes_from_the_side_view_callbacks_too() {
    let inside = view_after_callback(BRIGHT, None);
    assert_eq!(inside, Some(view(HOUSE, true)));
    let outside = view_after_callback("CB_SideView_SetBright_NierHome_Out", inside.as_ref());
    assert_eq!(outside, Some(view(HOUSE, false)));
}

#[test]
fn the_side_view_environment_calls_flip_the_last_known_chain() {
    let last = view(BAR, false);
    assert_eq!(
        view_after_callback("SideView_ENV_ON", Some(&last)),
        Some(view(BAR, true))
    );
    assert_eq!(
        view_after_callback("SideView_ENV_OFF", Some(&view(BAR, true))),
        Some(view(BAR, false))
    );
    assert_eq!(view_after_callback("SideView_ENV_ON", None), None);
    assert_eq!(view_after_callback("From_1F_to_2F_01", Some(&last)), None);
}

#[test]
fn a_zone_change_while_inside_makes_the_state_outside() {
    let inside = Some(view(HOUSE, true));
    assert_eq!(
        view_after_phase_check(inside.clone(), Some(3), Some(4)),
        Some(view(HOUSE, false))
    );
    assert_eq!(
        view_after_phase_check(inside.clone(), Some(3), Some(3)),
        inside
    );
    assert_eq!(
        view_after_phase_check(inside.clone(), None, Some(3)),
        inside
    );
}
