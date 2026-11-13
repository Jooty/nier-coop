use replicant_game::stay_together::FollowRole;
use replicant_game::story_calls::{PartnerPlace, SceneScope, StoryCallKind};
use replicant_game::story_relay::{
    CallKey, FireAction, FireFacts, GATHER_RELEASE_REMEMBERED_MS, GatherAdmission,
    GatherAloneReason, GatherFacts, GatherInputs, GatherOutcome, GatherStayReason, GatherVerdict,
    MAX_SCENE_CALLS_HELD_FOR_PARTNER, SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS, SceneGatherHold,
    StoryRelayLedger, gather_verdict,
};
use replicant_proto::{GatherReply, StoryCall};

const PHASE: u16 = 12;
const KEY: CallKey = CallKey {
    function: 7,
    args: 9,
};

fn call(function: &str) -> StoryCall {
    StoryCall {
        phase: PHASE,
        pack: 1,
        function: function.to_owned(),
        args: Vec::new(),
    }
}

fn inputs(now_ms: u64, place: PartnerPlace) -> GatherInputs {
    GatherInputs {
        now_ms,
        connected: true,
        partner_in_play: true,
        partner_place: place,
        local_phase: Some(PHASE),
    }
}

const FAR: PartnerPlace = PartnerPlace::TooFar { units: 400.0 };

fn holding(function: &str) -> SceneGatherHold {
    let mut hold = holding_unanswered(function);
    assert!(hold.partner_answered(PHASE, GatherReply::Ready));
    hold
}

fn holding_unanswered(function: &str) -> SceneGatherHold {
    let mut hold = SceneGatherHold::new();
    assert!(hold.hold(call(function), 1_000));
    hold
}

#[test]
fn a_partner_who_is_near_but_has_not_answered_is_still_waited_for() {
    let mut hold = holding_unanswered("SceneA");
    assert!(
        hold.step(&inputs(1_500, PartnerPlace::Same))
            .released
            .is_empty()
    );
    assert!(hold.partner_answered(PHASE, GatherReply::Ready));
    let step = hold.step(&inputs(1_600, PartnerPlace::Same));
    assert_eq!(step.released[0].outcome, GatherOutcome::PartnerArrived);
}

#[test]
fn a_ready_partner_who_is_not_seen_here_yet_is_still_waited_for() {
    let mut hold = holding("SceneA");
    assert!(hold.step(&inputs(1_500, FAR)).released.is_empty());
}

#[test]
fn a_partner_who_stays_releases_the_call_at_once_with_the_reason() {
    for reply in [
        GatherReply::InAnotherArea,
        GatherReply::InOwnTalk,
        GatherReply::InOwnScene,
        GatherReply::Downed,
        GatherReply::SideViewNotGiven,
        GatherReply::DidNotArrive,
    ] {
        let mut hold = holding_unanswered("SceneA");
        hold.partner_answered(PHASE, reply);
        let step = hold.step(&inputs(1_050, FAR));
        assert_eq!(
            step.released[0].outcome,
            GatherOutcome::RunAlone(GatherAloneReason::PartnerStays(reply)),
            "{reply:?}"
        );
    }
}

#[test]
fn an_unanswered_call_never_waits_past_the_time_limit() {
    let mut hold = holding_unanswered("SceneA");
    let limit = 1_000 + SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS;
    assert!(hold.step(&inputs(limit - 1, FAR)).released.is_empty());
    let step = hold.step(&inputs(limit, FAR));
    assert_eq!(
        step.released[0].outcome,
        GatherOutcome::RunAlone(GatherAloneReason::TimeLimit)
    );
    assert!(hold.step(&inputs(limit + 1, FAR)).released.is_empty());
}

#[test]
fn an_answer_for_another_phase_or_with_nothing_held_is_ignored() {
    let mut hold = holding_unanswered("SceneA");
    assert!(!hold.partner_answered(PHASE + 1, GatherReply::Downed));
    assert!(hold.step(&inputs(1_100, FAR)).released.is_empty());
    assert!(!SceneGatherHold::new().partner_answered(PHASE, GatherReply::Ready));
}

#[test]
fn a_call_runs_with_the_partner_once_the_partner_arrives() {
    let mut hold = holding("SceneA");
    assert!(hold.step(&inputs(1_500, FAR)).released.is_empty());
    let step = hold.step(&inputs(1_900, PartnerPlace::Same));
    assert_eq!(step.released.len(), 1);
    assert_eq!(step.released[0].outcome, GatherOutcome::PartnerArrived);
    assert_eq!(step.released[0].waited_ms, 900);
    assert!(step.forgotten.is_empty());
}

#[test]
fn a_call_runs_alone_when_the_time_limit_passes() {
    let mut hold = holding("SceneA");
    let before = 1_000 + SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS - 1;
    assert!(hold.step(&inputs(before, FAR)).released.is_empty());
    let step = hold.step(&inputs(before + 1, FAR));
    assert_eq!(
        step.released[0].outcome,
        GatherOutcome::RunAlone(GatherAloneReason::TimeLimit)
    );
}

#[test]
fn a_partner_who_arrives_at_the_deadline_still_counts_as_arrived() {
    let mut hold = holding("SceneA");
    let at_deadline = 1_000 + SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS;
    let step = hold.step(&inputs(at_deadline, PartnerPlace::Same));
    assert_eq!(step.released[0].outcome, GatherOutcome::PartnerArrived);
}

#[test]
fn a_call_runs_alone_at_once_when_the_partner_is_gone() {
    let cases = [
        (
            GatherInputs {
                partner_in_play: false,
                ..inputs(1_100, FAR)
            },
            GatherAloneReason::PartnerNotInPlay,
        ),
        (
            GatherInputs {
                connected: false,
                ..inputs(1_100, FAR)
            },
            GatherAloneReason::ConnectionLost,
        ),
        (
            inputs(1_100, PartnerPlace::OtherPhase),
            GatherAloneReason::PartnerInAnotherPhase,
        ),
    ];
    for (input, reason) in cases {
        let step = holding("SceneA").step(&input);
        assert_eq!(
            step.released[0].outcome,
            GatherOutcome::RunAlone(reason),
            "{reason:?}"
        );
    }
}

#[test]
fn a_call_is_forgotten_when_the_host_changes_phase() {
    let mut hold = holding("SceneA");
    let step = hold.step(&GatherInputs {
        local_phase: Some(PHASE + 1),
        ..inputs(1_100, PartnerPlace::Same)
    });
    assert!(step.released.is_empty());
    assert_eq!(step.forgotten, vec![call("SceneA")]);
    assert!(hold.is_idle());
}

#[test]
fn a_call_is_forgotten_when_the_host_has_no_phase() {
    let mut hold = holding("SceneA");
    let step = hold.step(&GatherInputs {
        local_phase: None,
        ..inputs(1_100, FAR)
    });
    assert_eq!(step.forgotten.len(), 1);
}

#[test]
fn a_released_call_is_not_held_again_on_its_next_pop() {
    let mut hold = holding("SceneA");
    hold.step(&inputs(1_100, PartnerPlace::Same));
    assert_eq!(
        hold.admit(PHASE, "SceneA", 1_200),
        GatherAdmission::JustReleased
    );
    assert_eq!(hold.admit(PHASE, "SceneA", 1_300), GatherAdmission::Open);
}

#[test]
fn the_memory_of_a_release_expires() {
    let mut hold = holding("SceneA");
    hold.step(&inputs(1_100, PartnerPlace::Same));
    let later = 1_100 + GATHER_RELEASE_REMEMBERED_MS;
    assert_eq!(hold.admit(PHASE, "SceneA", later), GatherAdmission::Open);
}

#[test]
fn the_memory_of_a_release_ends_on_a_phase_change() {
    let mut hold = holding("SceneA");
    hold.step(&inputs(1_100, PartnerPlace::Same));
    hold.step(&GatherInputs {
        local_phase: Some(PHASE + 1),
        ..inputs(1_200, FAR)
    });
    assert!(hold.is_idle());
    assert_eq!(hold.admit(PHASE, "SceneA", 1_300), GatherAdmission::Open);
}

#[test]
fn the_memory_of_a_release_is_per_function() {
    let mut hold = holding("SceneA");
    hold.step(&inputs(1_100, PartnerPlace::Same));
    assert_eq!(hold.admit(PHASE, "SceneB", 1_200), GatherAdmission::Open);
}

#[test]
fn two_different_calls_wait_and_release_independently() {
    let mut hold = holding("SceneA");
    assert!(hold.hold(call("SceneB"), 1_500));
    let first = hold.step(&inputs(3_000, FAR));
    assert_eq!(first.released.len(), 1);
    assert_eq!(first.released[0].call.function, "SceneA");
    let second = hold.step(&inputs(3_500, FAR));
    assert_eq!(second.released.len(), 1);
    assert_eq!(second.released[0].call.function, "SceneB");
}

#[test]
fn the_same_function_in_the_same_phase_is_held_once() {
    let mut hold = holding("SceneA");
    assert_eq!(
        hold.admit(PHASE, "SceneA", 1_100),
        GatherAdmission::AlreadyWaiting
    );
    assert!(!hold.hold(call("SceneA"), 1_100));
}

#[test]
fn the_hold_has_a_capacity() {
    let mut hold = SceneGatherHold::new();
    for number in 0..MAX_SCENE_CALLS_HELD_FOR_PARTNER {
        assert!(hold.hold(call(&format!("Scene{number}")), 0));
    }
    assert!(!hold.has_room());
    assert_eq!(hold.admit(PHASE, "Other", 10), GatherAdmission::Full);
    assert!(!hold.hold(call("Other"), 10));
}

fn fire_facts(role: FollowRole) -> FireFacts {
    FireFacts {
        enabled: true,
        connected: true,
        role,
        local_in_play: true,
        partner_in_same_phase: true,
        kind: Some(StoryCallKind::Story),
        scope: SceneScope::SharedScene,
        partner_here: false,
        owned_elsewhere: false,
        args_relayable: true,
        partner_too_far: true,
        can_gather_partner: true,
        each_run_counts: false,
    }
}

fn verdict(facts: &FireFacts) -> FireAction {
    StoryRelayLedger::new().on_fire(0x1000, facts, KEY, 0)
}

#[test]
fn the_host_holds_a_shared_scene_for_a_far_partner() {
    assert_eq!(
        verdict(&fire_facts(FollowRole::Host)),
        FireAction::HoldForGatheredPartner
    );
}

#[test]
fn the_guest_never_holds_a_shared_scene() {
    assert_eq!(
        verdict(&fire_facts(FollowRole::Guest)),
        FireAction::RunWithoutPartner
    );
}

#[test]
fn a_partner_in_another_phase_or_unknown_is_not_pulled() {
    let facts = FireFacts {
        partner_too_far: false,
        partner_in_same_phase: false,
        ..fire_facts(FollowRole::Host)
    };
    assert_eq!(verdict(&facts), FireAction::RunWithoutPartner);
    let unknown = FireFacts {
        partner_too_far: false,
        ..fire_facts(FollowRole::Host)
    };
    assert_eq!(verdict(&unknown), FireAction::RunWithoutPartner);
}

#[test]
fn arguments_that_cannot_be_carried_are_not_held() {
    let facts = FireFacts {
        args_relayable: false,
        ..fire_facts(FollowRole::Host)
    };
    assert_eq!(verdict(&facts), FireAction::RunWithoutPartner);
}

#[test]
fn a_closed_hold_leaves_the_scene_to_run_alone() {
    let facts = FireFacts {
        can_gather_partner: false,
        ..fire_facts(FollowRole::Host)
    };
    assert_eq!(verdict(&facts), FireAction::RunWithoutPartner);
}

#[test]
fn a_partner_who_is_here_is_not_held_for() {
    let facts = FireFacts {
        partner_here: true,
        partner_too_far: false,
        ..fire_facts(FollowRole::Host)
    };
    assert_eq!(verdict(&facts), FireAction::RunAndAnnounce);
}

fn free_guest() -> GatherFacts {
    GatherFacts {
        same_phase: true,
        in_side_view: false,
        starter_in_side_view: false,
        in_own_talk: false,
        in_event_scene: false,
        downed: false,
    }
}

#[test]
fn a_free_player_in_the_same_phase_comes_to_the_starter() {
    assert_eq!(
        gather_verdict(&free_guest()),
        GatherVerdict::Come {
            across_side_view: false
        }
    );
}

#[test]
fn a_camera_view_difference_in_either_direction_is_a_move_across_it() {
    for (in_side_view, starter_in_side_view) in [(true, false), (false, true)] {
        let facts = GatherFacts {
            in_side_view,
            starter_in_side_view,
            ..free_guest()
        };
        assert_eq!(
            gather_verdict(&facts),
            GatherVerdict::Come {
                across_side_view: true
            }
        );
    }
    let both_inside = GatherFacts {
        in_side_view: true,
        starter_in_side_view: true,
        ..free_guest()
    };
    assert_eq!(
        gather_verdict(&both_inside),
        GatherVerdict::Come {
            across_side_view: false
        }
    );
}

#[test]
fn every_reason_keeps_the_player_where_it_is() {
    let cases = [
        (
            GatherFacts {
                same_phase: false,
                ..free_guest()
            },
            GatherStayReason::OtherPhase,
        ),
        (
            GatherFacts {
                in_own_talk: true,
                ..free_guest()
            },
            GatherStayReason::InOwnTalk,
        ),
        (
            GatherFacts {
                in_event_scene: true,
                ..free_guest()
            },
            GatherStayReason::InOwnScene,
        ),
        (
            GatherFacts {
                downed: true,
                ..free_guest()
            },
            GatherStayReason::Downed,
        ),
    ];
    for (facts, reason) in cases {
        assert_eq!(gather_verdict(&facts), GatherVerdict::Stay(reason));
        assert_ne!(reason.reply(), GatherReply::Ready);
    }
}

#[test]
fn a_talk_of_ones_own_is_kept_even_across_a_camera_view() {
    let facts = GatherFacts {
        in_own_talk: true,
        starter_in_side_view: true,
        ..free_guest()
    };
    assert_eq!(
        gather_verdict(&facts),
        GatherVerdict::Stay(GatherStayReason::InOwnTalk)
    );
}
