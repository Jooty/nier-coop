use replicant_game::scene_follow::{
    FollowStep, InjectedCallbacks, RecentCallbacks, SCENE_FOLLOW_AFTER_DIALOG_CLOSE_MS,
    SCENE_FOLLOW_DUPLICATE_WINDOW_MS, SCENE_FOLLOW_GIVE_UP_MS, SCENE_FOLLOW_GRACE_MS,
    SCENE_FOLLOW_LOOKBACK_MS, SceneFollowWatch,
};
use replicant_game::ui_scenes::PAUSE_MENU_CLOSE_MAX_REQUESTS_PER_SCENE_START;

const SCENE_CALLBACK: &str = "EID_0050_a0030g_End";

fn watch_with_pending(now_ms: u64) -> SceneFollowWatch {
    let mut watch = SceneFollowWatch::new();
    watch.receive(vec![SCENE_CALLBACK.to_owned()], now_ms);
    watch
}

#[test]
fn only_event_callbacks_inside_the_lookback_are_sent() {
    let mut recent = RecentCallbacks::new();
    recent.note("Impact_Effect_01", 900);
    recent.note("EID_0050_a0020_End", 1_000);
    recent.note(SCENE_CALLBACK, 5_000);
    assert_eq!(recent.names_before_scene(5_500), vec![SCENE_CALLBACK]);
    assert!(
        recent
            .names_before_scene(5_000 + SCENE_FOLLOW_LOOKBACK_MS + 1)
            .is_empty()
    );
}

#[test]
fn a_guest_that_enters_its_own_scene_in_time_is_not_forced() {
    let mut watch = watch_with_pending(10_000);
    assert_eq!(watch.step(false, false, false, 10_500), FollowStep::Wait);
    assert_eq!(
        watch.step(true, false, false, 11_000),
        FollowStep::Followed { waited_ms: 1_000 }
    );
    assert!(!watch.has_pending());
}

#[test]
fn a_scene_the_guest_entered_just_before_the_message_counts() {
    let mut watch = SceneFollowWatch::new();
    watch.step(false, false, false, 9_000);
    watch.step(true, false, false, 9_500);
    watch.receive(vec![SCENE_CALLBACK.to_owned()], 10_000);
    assert_eq!(
        watch.step(true, false, false, 10_100),
        FollowStep::Followed { waited_ms: 100 }
    );
}

#[test]
fn a_guest_that_never_enters_gets_the_callback_once_after_the_grace() {
    let mut watch = watch_with_pending(0);
    assert_eq!(
        watch.step(false, false, false, SCENE_FOLLOW_GRACE_MS - 1),
        FollowStep::Wait
    );
    assert_eq!(
        watch.step(false, false, false, SCENE_FOLLOW_GRACE_MS),
        FollowStep::Inject {
            waited_ms: SCENE_FOLLOW_GRACE_MS,
            callbacks: vec![SCENE_CALLBACK.to_owned()],
        }
    );
    assert_eq!(
        watch.step(false, false, false, SCENE_FOLLOW_GRACE_MS + 100),
        FollowStep::Wait
    );
}

#[test]
fn an_open_prompt_is_closed_before_the_callback_is_injected() {
    let mut watch = watch_with_pending(0);
    assert_eq!(
        watch.step(false, true, false, SCENE_FOLLOW_GRACE_MS),
        FollowStep::CloseDialog {
            waited_ms: SCENE_FOLLOW_GRACE_MS
        }
    );
    assert_eq!(
        watch.step(false, true, false, SCENE_FOLLOW_GRACE_MS + 100),
        FollowStep::Wait
    );
    let settled = SCENE_FOLLOW_GRACE_MS + SCENE_FOLLOW_AFTER_DIALOG_CLOSE_MS;
    assert!(matches!(
        watch.step(false, true, false, settled),
        FollowStep::Inject { .. }
    ));
}

#[test]
fn a_follow_that_never_lands_gives_up() {
    let mut watch = watch_with_pending(0);
    assert!(matches!(
        watch.step(false, false, false, SCENE_FOLLOW_GRACE_MS),
        FollowStep::Inject { .. }
    ));
    assert_eq!(
        watch.step(false, false, false, SCENE_FOLLOW_GIVE_UP_MS),
        FollowStep::GaveUp {
            waited_ms: SCENE_FOLLOW_GIVE_UP_MS
        }
    );
    assert_eq!(
        watch.step(false, false, false, SCENE_FOLLOW_GIVE_UP_MS + 1),
        FollowStep::Idle
    );
}

#[test]
fn the_injected_callback_runs_once_and_a_late_natural_fire_is_dropped() {
    let mut injected = InjectedCallbacks::new();
    injected.add(SCENE_CALLBACK, 1_000);
    assert!(!injected.is_duplicate("Other_Callback", 1_100));
    assert!(!injected.is_duplicate(SCENE_CALLBACK, 1_200));
    assert!(injected.is_duplicate(SCENE_CALLBACK, 5_000));
    assert!(!injected.is_duplicate(SCENE_CALLBACK, 1_000 + SCENE_FOLLOW_DUPLICATE_WINDOW_MS + 1));
}

#[test]
fn an_open_pause_menu_is_closed_at_once_without_the_grace() {
    let mut watch = watch_with_pending(0);
    assert_eq!(
        watch.step(false, false, true, 10),
        FollowStep::ClosePauseMenu { waited_ms: 10 }
    );
    assert_eq!(watch.step(false, false, false, 20), FollowStep::Wait);
    let settled = 10 + SCENE_FOLLOW_AFTER_DIALOG_CLOSE_MS;
    assert!(matches!(
        watch.step(false, false, false, settled),
        FollowStep::Inject { .. }
    ));
}

#[test]
fn a_pause_menu_that_stays_open_is_closed_only_twice_per_scene_start() {
    let mut watch = watch_with_pending(0);
    let mut closes = 0_u8;
    for tick in 0..20 {
        let now = tick * SCENE_FOLLOW_AFTER_DIALOG_CLOSE_MS;
        if matches!(
            watch.step(false, false, true, now),
            FollowStep::ClosePauseMenu { .. }
        ) {
            closes += 1;
        }
    }
    assert_eq!(closes, PAUSE_MENU_CLOSE_MAX_REQUESTS_PER_SCENE_START);
}

#[test]
fn the_close_cap_is_lifted_while_the_host_is_still_in_the_scene() {
    let mut watch = watch_with_pending(0);
    watch.set_host_in_scene(true);
    let mut closes = 0_u8;
    for tick in 0..20 {
        let now = tick * SCENE_FOLLOW_AFTER_DIALOG_CLOSE_MS;
        if matches!(
            watch.step(false, false, true, now),
            FollowStep::ClosePauseMenu { .. }
        ) {
            closes += 1;
        }
    }
    assert!(closes > PAUSE_MENU_CLOSE_MAX_REQUESTS_PER_SCENE_START);
}

#[test]
fn an_announcement_with_no_names_never_injects_but_still_closes_the_pause_menu() {
    let mut watch = SceneFollowWatch::new();
    watch.receive(Vec::new(), 0);
    assert!(watch.has_pending());
    assert_eq!(
        watch.step(false, false, true, 10),
        FollowStep::ClosePauseMenu { waited_ms: 10 }
    );
    for now in (1_000..SCENE_FOLLOW_GIVE_UP_MS).step_by(500) {
        assert!(!matches!(
            watch.step(false, false, false, now),
            FollowStep::Inject { .. }
        ));
    }
    assert!(matches!(
        watch.step(false, false, false, SCENE_FOLLOW_GIVE_UP_MS),
        FollowStep::GaveUp { .. }
    ));
}

#[test]
fn an_announcement_with_no_names_closes_a_third_time_while_the_host_is_in_the_scene() {
    let mut watch = SceneFollowWatch::new();
    watch.receive(Vec::new(), 0);
    watch.set_host_in_scene(true);
    let mut closes = 0_u8;
    for tick in 0..20 {
        let now = tick * SCENE_FOLLOW_AFTER_DIALOG_CLOSE_MS;
        let step = watch.step(false, false, true, now);
        assert!(!matches!(step, FollowStep::Inject { .. }));
        if matches!(step, FollowStep::ClosePauseMenu { .. }) {
            closes += 1;
        }
    }
    assert!(closes > PAUSE_MENU_CLOSE_MAX_REQUESTS_PER_SCENE_START);
}
