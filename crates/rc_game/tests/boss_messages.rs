use rc_game::boss_messages::{
    GuestFacts, GuestVerdict, LIFE_CALLBACK_KIND, LifeCallbackEntry, MirrorVerdict, RaisedSeqs,
    SkipReason, guest_verdict, held_hp_to_restore, life_callback_targets_group, mirror_verdict,
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
fn each_numbered_raise_runs_once_even_for_the_same_message_in_a_row() {
    let mut seqs = RaisedSeqs::new();
    assert!(seqs.allow(1));
    assert!(!seqs.allow(1));
    assert!(seqs.allow(2));
    assert!(seqs.allow(3));
}

#[test]
fn clearing_forgets_recent_numbers() {
    let mut seqs = RaisedSeqs::new();
    assert!(seqs.allow(5));
    seqs.clear();
    assert!(seqs.allow(5));
}

#[test]
fn only_a_registered_life_callback_naming_the_group_counts() {
    let words = [3, 0, 1001, 4, 0, 0, 0, 0];
    let other = [9, 0, 7, 4, 0, 0, 0, 0];
    let entries = [
        LifeCallbackEntry {
            kind: LIFE_CALLBACK_KIND,
            data_words: &words,
        },
        LifeCallbackEntry {
            kind: 0x10,
            data_words: &other,
        },
    ];
    assert!(life_callback_targets_group(&entries, 1001));
    assert!(!life_callback_targets_group(&entries, 7));
    assert!(!life_callback_targets_group(&[], 1001));
}

#[test]
fn a_local_hit_never_lowers_a_held_copys_hp() {
    assert_eq!(held_hp_to_restore(500, 420), Some(500));
    assert_eq!(held_hp_to_restore(500, 500), None);
    assert_eq!(held_hp_to_restore(500, 0), None);
    assert_eq!(held_hp_to_restore(0, 0), None);
    assert_eq!(held_hp_to_restore(500, 600), None);
}
