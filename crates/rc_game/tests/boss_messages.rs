use rc_game::boss_messages::{
    GuestFacts, GuestVerdict, MESSAGE_DEDUPE_WINDOW_MS, MessageDedupe, MirrorVerdict, SkipReason,
    guest_verdict, held_hp_to_restore, mirror_verdict,
};

fn facts(message: u32) -> GuestFacts {
    GuestFacts {
        enabled: true,
        is_guest: true,
        local_in_play: true,
        local_phase: Some(12),
        message_phase: 12,
        message,
    }
}

#[test]
fn only_the_boss_answer_range_is_mirrored() {
    assert_eq!(mirror_verdict(241), MirrorVerdict::Mirror);
    assert_eq!(mirror_verdict(278), MirrorVerdict::Mirror);
    assert_eq!(mirror_verdict(299), MirrorVerdict::Mirror);
    assert_eq!(mirror_verdict(240), MirrorVerdict::OutsideRange);
    assert_eq!(mirror_verdict(300), MirrorVerdict::OutsideRange);
}

#[test]
fn the_players_own_posts_are_never_mirrored() {
    for message in [210, 213, 215, 470, 471, 472, 473] {
        assert_eq!(mirror_verdict(message), MirrorVerdict::NeverMirrored);
    }
}

#[test]
fn the_guest_raises_a_mirrored_message_of_its_own_phase() {
    assert_eq!(guest_verdict(&facts(243)), GuestVerdict::Raise);
}

#[test]
fn the_guest_skips_what_it_must_not_raise() {
    let mut other_phase = facts(243);
    other_phase.message_phase = 13;
    assert_eq!(
        guest_verdict(&other_phase),
        GuestVerdict::Skip(SkipReason::OtherPhase)
    );
    let mut off = facts(243);
    off.enabled = false;
    assert_eq!(
        guest_verdict(&off),
        GuestVerdict::Skip(SkipReason::Disabled)
    );
    let mut host = facts(243);
    host.is_guest = false;
    assert_eq!(
        guest_verdict(&host),
        GuestVerdict::Skip(SkipReason::NotGuest)
    );
    let mut loading = facts(243);
    loading.local_in_play = false;
    assert_eq!(
        guest_verdict(&loading),
        GuestVerdict::Skip(SkipReason::NotInPlay)
    );
    assert_eq!(
        guest_verdict(&facts(210)),
        GuestVerdict::Skip(SkipReason::NotMirrored)
    );
    let mut unknown_phase = facts(243);
    unknown_phase.local_phase = None;
    assert_eq!(
        guest_verdict(&unknown_phase),
        GuestVerdict::Skip(SkipReason::OtherPhase)
    );
}

#[test]
fn the_same_number_twice_in_a_tick_is_one_raise() {
    let mut dedupe = MessageDedupe::new();
    assert!(dedupe.allow(243, 1_000));
    assert!(!dedupe.allow(243, 1_016));
    assert!(dedupe.allow(244, 1_016));
    assert!(dedupe.allow(243, 1_000 + MESSAGE_DEDUPE_WINDOW_MS));
}

#[test]
fn clearing_forgets_recent_numbers() {
    let mut dedupe = MessageDedupe::new();
    assert!(dedupe.allow(243, 5));
    dedupe.clear();
    assert!(dedupe.allow(243, 6));
}

#[test]
fn a_local_hit_never_lowers_a_held_copys_hp() {
    assert_eq!(held_hp_to_restore(500, 420), Some(500));
    assert_eq!(held_hp_to_restore(500, 500), None);
    assert_eq!(held_hp_to_restore(500, 0), None);
    assert_eq!(held_hp_to_restore(0, 0), None);
    assert_eq!(held_hp_to_restore(500, 600), None);
}
