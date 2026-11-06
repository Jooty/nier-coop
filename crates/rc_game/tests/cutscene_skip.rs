use rc_game::cutscene_skip::{
    ASK_RESEND_INTERVAL_MS, CutsceneSkipTracker, PARTNER_ASK_LIFETIME_MS, SkipVerdict,
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
fn asks_arriving_outside_a_scene_are_dropped() {
    let mut tracker = CutsceneSkipTracker::new();
    assert!(!tracker.partner_asked(0));
    tracker.observe_scene(true, false);
    assert_eq!(tracker.step(true, true, 5).verdict, SkipVerdict::Hold);
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
