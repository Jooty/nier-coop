use replicant_game::scene_follow::{AnnounceDrop, AnnounceVerdict, ComingOver, ComingStep};
use replicant_game::story_calls::PartnerPlace;
use replicant_game::story_relay::{
    GatherStayReason, GatherVerdict, SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS,
};

const FAR: PartnerPlace = PartnerPlace::TooFar { units: 300.0 };
const COME: GatherVerdict = GatherVerdict::Come {
    across_side_view: false,
};

fn names() -> Vec<String> {
    vec![String::from("SceneStart")]
}

fn announce(
    coming: &mut ComingOver,
    place: PartnerPlace,
    verdict: GatherVerdict,
) -> AnnounceVerdict {
    coming.on_announce(place, verdict, false, names(), 100)
}

#[test]
fn a_far_free_guest_comes_over_then_follows() {
    let mut coming = ComingOver::new();
    assert_eq!(
        announce(&mut coming, FAR, COME),
        AnnounceVerdict::ComeOver { move_needed: true }
    );
    assert_eq!(coming.step(FAR, true, 600), ComingStep::Waiting);
    assert_eq!(
        coming.step(PartnerPlace::Same, true, 900),
        ComingStep::Arrived {
            waited_ms: 800,
            callbacks: names()
        }
    );
    assert!(!coming.is_coming());
}

#[test]
fn every_stay_reason_drops_the_announce() {
    for reason in [
        GatherStayReason::OtherPhase,
        GatherStayReason::InOwnTalk,
        GatherStayReason::InOwnScene,
        GatherStayReason::Downed,
    ] {
        let mut coming = ComingOver::new();
        assert_eq!(
            announce(&mut coming, FAR, GatherVerdict::Stay(reason)),
            AnnounceVerdict::Drop(AnnounceDrop::Stay(reason))
        );
        assert!(!coming.is_coming());
    }
}

#[test]
fn a_host_in_another_phase_or_unknown_is_dropped() {
    for place in [PartnerPlace::OtherPhase, PartnerPlace::Unknown] {
        let mut coming = ComingOver::new();
        assert_eq!(
            announce(&mut coming, place, COME),
            AnnounceVerdict::Drop(AnnounceDrop::HostNotFarAway)
        );
    }
}

#[test]
fn a_guest_who_does_not_arrive_in_time_gives_up() {
    let mut coming = ComingOver::new();
    announce(&mut coming, FAR, COME);
    let late = 100 + SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS;
    assert_eq!(coming.step(FAR, true, late - 1), ComingStep::Waiting);
    assert_eq!(
        coming.step(FAR, true, late),
        ComingStep::GaveUp {
            waited_ms: SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS
        }
    );
    assert_eq!(coming.step(FAR, true, late + 1), ComingStep::Idle);
}

#[test]
fn a_guest_already_in_range_follows_at_once() {
    let mut coming = ComingOver::new();
    assert_eq!(
        announce(
            &mut coming,
            PartnerPlace::Same,
            GatherVerdict::Stay(GatherStayReason::Downed)
        ),
        AnnounceVerdict::FollowNow
    );
    assert!(!coming.is_coming());
}

#[test]
fn a_second_announce_during_the_move_starts_no_second_move() {
    let mut coming = ComingOver::new();
    announce(&mut coming, FAR, COME);
    assert_eq!(
        announce(&mut coming, FAR, COME),
        AnnounceVerdict::AlreadyComing
    );
}

#[test]
fn a_move_that_just_began_for_another_reason_is_not_repeated() {
    let mut coming = ComingOver::new();
    assert_eq!(
        coming.on_announce(FAR, COME, true, names(), 100),
        AnnounceVerdict::ComeOver { move_needed: false }
    );
}

#[test]
fn a_guest_who_is_near_but_still_settling_does_not_follow_yet() {
    let mut coming = ComingOver::new();
    announce(&mut coming, FAR, COME);
    assert_eq!(
        coming.step(PartnerPlace::Same, false, 500),
        ComingStep::Waiting
    );
    assert_eq!(
        coming.step(PartnerPlace::Same, true, 700),
        ComingStep::Arrived {
            waited_ms: 600,
            callbacks: names()
        }
    );
}

#[test]
fn a_guest_who_never_settles_gives_up_at_the_limit() {
    let mut coming = ComingOver::new();
    announce(&mut coming, FAR, COME);
    let late = 100 + SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS;
    assert_eq!(
        coming.step(PartnerPlace::Same, false, late),
        ComingStep::GaveUp {
            waited_ms: SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS
        }
    );
}
