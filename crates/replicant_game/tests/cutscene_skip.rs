use replicant_game::cutscene_skip::{
    ASK_RESEND_INTERVAL_MS, CutsceneSkipTracker, PARTNER_ASK_LIFETIME_MS,
    SCENE_START_HOLD_RESTART_GAP_MS, SCENE_START_HOLD_TIMEOUT_MS, SceneStartGate, SkipVerdict,
    StartVerdict,
};

fn in_scene() -> CutsceneSkipTracker {
    let mut tracker = CutsceneSkipTracker::new();
    tracker.observe_scene(true, false);
    tracker
}

#[test]
fn a_press_with_the_partner_not_asking_holds() {
    let mut tracker = in_scene();
    let step = tracker.step(true, true, 0);
    assert_eq!(step.verdict, SkipVerdict::Hold);
    assert!(step.newly_asked);
    let again = tracker.step(true, true, 100);
    assert_eq!(again.verdict, SkipVerdict::Hold);
    assert!(!again.newly_asked);
}

#[test]
fn nothing_happens_before_anyone_presses() {
    let mut tracker = in_scene();
    assert_eq!(
        tracker.step(false, true, 0).verdict,
        SkipVerdict::PassThrough
    );
}

#[test]
fn the_last_to_ask_lets_the_game_skip() {
    let mut tracker = in_scene();
    assert!(tracker.partner_asked(0));
    let step = tracker.step(true, true, 50);
    assert_eq!(step.verdict, SkipVerdict::LetGameSkip);
}

#[test]
fn the_first_to_ask_skips_when_the_partner_asks_later() {
    let mut tracker = in_scene();
    assert_eq!(tracker.step(true, true, 0).verdict, SkipVerdict::Hold);
    tracker.partner_asked(300);
    assert_eq!(tracker.step(false, true, 316).verdict, SkipVerdict::SkipNow);
    assert_eq!(
        tracker.step(false, true, 333).verdict,
        SkipVerdict::PassThrough
    );
}

#[test]
fn a_partner_outside_a_cutscene_is_not_waited_for() {
    let mut tracker = in_scene();
    assert_eq!(
        tracker.step(true, false, 0).verdict,
        SkipVerdict::LetGameSkip
    );
}

#[test]
fn a_partner_who_leaves_while_held_releases_the_skip() {
    let mut tracker = in_scene();
    assert_eq!(tracker.step(true, true, 0).verdict, SkipVerdict::Hold);
    assert_eq!(tracker.step(false, false, 40).verdict, SkipVerdict::SkipNow);
}

#[test]
fn the_partners_ask_expires_without_resends() {
    let mut tracker = in_scene();
    tracker.partner_asked(0);
    let late = PARTNER_ASK_LIFETIME_MS + 1;
    assert_eq!(tracker.step(true, true, late).verdict, SkipVerdict::Hold);
}

#[test]
fn a_resent_ask_is_not_announced_twice() {
    let mut tracker = in_scene();
    assert!(tracker.partner_asked(0));
    assert!(!tracker.partner_asked(ASK_RESEND_INTERVAL_MS));
    assert!(tracker.partner_asked(ASK_RESEND_INTERVAL_MS + PARTNER_ASK_LIFETIME_MS + 1));
}

#[test]
fn asks_never_carry_into_the_next_scene() {
    let mut tracker = in_scene();
    tracker.partner_asked(0);
    tracker.step(true, true, 10);
    assert!(tracker.observe_scene(false, false));
    assert!(tracker.observe_scene(true, false));
    assert!(!tracker.local_asked());
    assert_eq!(tracker.step(true, true, 20).verdict, SkipVerdict::Hold);
}

#[test]
fn a_scene_starting_straight_after_a_skip_starts_clean() {
    let mut tracker = CutsceneSkipTracker::new();
    tracker.observe_scene(true, false);
    tracker.partner_asked(0);
    tracker.step(true, true, 10);
    tracker.observe_scene(true, true);
    assert!(tracker.observe_scene(true, false));
    assert_eq!(tracker.step(true, true, 30).verdict, SkipVerdict::Hold);
}

#[test]
fn an_early_ask_that_is_too_old_by_the_scene_start_is_ignored() {
    let mut tracker = CutsceneSkipTracker::new();
    assert!(tracker.partner_asked(0));
    tracker.observe_scene(true, false);
    assert_eq!(
        tracker
            .step(true, true, PARTNER_ASK_LIFETIME_MS + 5)
            .verdict,
        SkipVerdict::Hold
    );
}

#[test]
fn the_ask_is_resent_until_the_scene_ends() {
    let mut tracker = in_scene();
    assert!(!tracker.resend_due(0));
    tracker.step(true, true, 0);
    assert!(tracker.resend_due(0));
    assert!(!tracker.resend_due(ASK_RESEND_INTERVAL_MS - 1));
    assert!(tracker.resend_due(ASK_RESEND_INTERVAL_MS));
    tracker.partner_asked(ASK_RESEND_INTERVAL_MS);
    tracker.step(false, true, ASK_RESEND_INTERVAL_MS);
    assert!(tracker.resend_due(ASK_RESEND_INTERVAL_MS * 3));
    tracker.observe_scene(false, false);
    assert!(!tracker.resend_due(ASK_RESEND_INTERVAL_MS * 6));
}

#[test]
fn a_partner_ask_ends_when_the_partner_leaves_the_scene() {
    let mut tracker = in_scene();
    tracker.observe_partner_in_cutscene(true);
    tracker.partner_asked(0);
    tracker.observe_partner_in_cutscene(false);
    assert!(!tracker.partner_ask_alive(1));
    assert_eq!(tracker.step(true, true, 2).verdict, SkipVerdict::Hold);
}

#[test]
fn an_ask_just_before_the_local_scene_starts_is_kept() {
    let mut tracker = CutsceneSkipTracker::new();
    tracker.partner_asked(100);
    tracker.observe_scene(true, false);
    assert!(tracker.partner_ask_alive(200));
    assert_eq!(
        tracker.step(true, true, 200).verdict,
        SkipVerdict::LetGameSkip
    );
}

fn primed_gate() -> SceneStartGate {
    let mut gate = SceneStartGate::new();
    gate.observe_host(false);
    gate
}

#[test]
fn the_guests_scene_waits_until_the_host_enters_one() {
    let mut gate = primed_gate();
    assert_eq!(gate.ask(0), StartVerdict::Hold { first: true });
    assert_eq!(gate.ask(16), StartVerdict::Hold { first: false });
    gate.observe_host(true);
    assert_eq!(gate.ask(500), StartVerdict::Start { waited_ms: 500 });
}

#[test]
fn a_host_scene_that_came_first_lets_the_guests_scene_start_at_once() {
    let mut gate = primed_gate();
    gate.observe_host(true);
    gate.observe_host(false);
    assert_eq!(gate.ask(9000), StartVerdict::Start { waited_ms: 0 });
}

#[test]
fn one_host_scene_pays_for_one_guest_scene() {
    let mut gate = primed_gate();
    gate.observe_host(true);
    gate.observe_host(false);
    assert_eq!(gate.ask(0), StartVerdict::Start { waited_ms: 0 });
    assert_eq!(gate.ask(5000), StartVerdict::Hold { first: true });
}

#[test]
fn a_scene_the_host_never_enters_starts_after_the_timeout() {
    let mut gate = primed_gate();
    assert_eq!(gate.ask(0), StartVerdict::Hold { first: true });
    let mut now = 0;
    while now + 16 < SCENE_START_HOLD_TIMEOUT_MS {
        now += 16;
        assert_eq!(gate.ask(now), StartVerdict::Hold { first: false });
    }
    assert_eq!(
        gate.ask(SCENE_START_HOLD_TIMEOUT_MS),
        StartVerdict::StartAfterTimeout {
            waited_ms: SCENE_START_HOLD_TIMEOUT_MS
        }
    );
}

#[test]
fn a_timed_out_scene_does_not_use_up_the_hosts_next_scene() {
    let mut gate = primed_gate();
    gate.ask(0);
    let mut now = 0;
    while now < SCENE_START_HOLD_TIMEOUT_MS {
        now += 16;
        gate.ask(now);
    }
    gate.observe_host(true);
    gate.observe_host(false);
    assert_eq!(gate.ask(now + 100), StartVerdict::Start { waited_ms: 0 });
}

#[test]
fn a_host_already_in_a_scene_when_the_gate_is_primed_does_not_count_as_a_new_one() {
    let mut gate = SceneStartGate::new();
    gate.observe_host(true);
    gate.observe_host(false);
    assert_eq!(gate.ask(0), StartVerdict::Hold { first: true });
}

#[test]
fn a_hold_that_was_left_alone_starts_its_timer_over() {
    let mut gate = primed_gate();
    assert_eq!(gate.ask(0), StartVerdict::Hold { first: true });
    let later = SCENE_START_HOLD_TIMEOUT_MS + SCENE_START_HOLD_RESTART_GAP_MS + 1;
    assert_eq!(gate.ask(later), StartVerdict::Hold { first: true });
}

#[test]
fn resetting_forgets_the_hosts_scenes() {
    let mut gate = primed_gate();
    gate.observe_host(true);
    gate.observe_host(false);
    gate.reset();
    gate.observe_host(false);
    assert_eq!(gate.ask(0), StartVerdict::Hold { first: true });
}
