use automata_game::story_gate::{
    CurrentPhase, SceneRequest, SceneRequestDecision, StoryRule, SubPhaseDecision,
    decide_own_travel_request, decide_scene_request, decide_sub_phase, foreign_scene_request_line,
    foreign_sub_phase_line, is_own_travel_request, own_travel_scene_request_line,
    refused_scene_request_line, refused_sub_phase_line, story_status_line,
};

const IN_THE_WORLD: CurrentPhase<'static> = CurrentPhase {
    id: 0x210,
    name: Some("made_up_step"),
};

fn foreign(phase_id: i32, phase_name: Option<&str>) -> SceneRequest<'_> {
    SceneRequest {
        ours: false,
        by_the_games_defeat_flow: false,
        phase_id,
        phase_name,
    }
}

fn of_the_defeat_flow(phase_id: i32, phase_name: Option<&str>) -> SceneRequest<'_> {
    SceneRequest {
        by_the_games_defeat_flow: true,
        ..foreign(phase_id, phase_name)
    }
}

fn ours(phase_id: i32) -> SceneRequest<'static> {
    SceneRequest {
        ours: true,
        by_the_games_defeat_flow: false,
        phase_id,
        phase_name: Some("anything"),
    }
}

#[test]
fn an_armed_gate_refuses_a_scene_request_for_another_phase() {
    assert_eq!(
        decide_scene_request(
            true,
            StoryRule::Hold,
            foreign(0x300, Some("next")),
            IN_THE_WORLD
        ),
        SceneRequestDecision::Refuse
    );
    assert_eq!(
        decide_scene_request(
            true,
            StoryRule::Hold,
            foreign(0xF30, Some("made_up_ending")),
            IN_THE_WORLD
        ),
        SceneRequestDecision::Refuse
    );
}

#[test]
fn our_own_requests_always_pass() {
    for armed in [true, false] {
        assert_eq!(
            decide_scene_request(armed, StoryRule::Hold, ours(0x300), IN_THE_WORLD),
            SceneRequestDecision::Pass
        );
        assert_eq!(
            decide_sub_phase(armed, StoryRule::Hold, ours(0x300), IN_THE_WORLD),
            SubPhaseDecision::Pass
        );
    }
}

#[test]
fn a_request_for_the_title_passes_and_disarms_the_gate() {
    for rule in [StoryRule::Hold, StoryRule::Allow] {
        assert_eq!(
            decide_scene_request(true, rule, foreign(0xF00, Some("START")), IN_THE_WORLD),
            SceneRequestDecision::PassToTitleAndDisarm
        );
        assert_eq!(
            decide_scene_request(true, rule, foreign(0xF00, None), IN_THE_WORLD),
            SceneRequestDecision::PassToTitleAndDisarm
        );
        assert_eq!(
            decide_scene_request(true, rule, foreign(0xF10, Some("made_up")), IN_THE_WORLD),
            SceneRequestDecision::PassToTitleAndDisarm
        );
        assert_eq!(
            decide_scene_request(true, rule, foreign(0xF10, None), IN_THE_WORLD),
            SceneRequestDecision::PassToTitleAndDisarm
        );
    }
}

#[test]
fn a_request_for_the_current_phase_and_name_passes() {
    assert_eq!(
        decide_scene_request(
            true,
            StoryRule::Hold,
            foreign(0x210, Some("made_up_step")),
            IN_THE_WORLD
        ),
        SceneRequestDecision::Pass
    );
}

#[test]
fn the_same_phase_under_another_name_or_no_name_is_refused() {
    for name in [Some("another_step"), Some(""), None] {
        assert_eq!(
            decide_scene_request(true, StoryRule::Hold, foreign(0x210, name), IN_THE_WORLD),
            SceneRequestDecision::Refuse
        );
    }
    let unreadable = CurrentPhase {
        id: 0x210,
        name: None,
    };
    assert_eq!(
        decide_scene_request(true, StoryRule::Hold, foreign(0x210, None), unreadable),
        SceneRequestDecision::Refuse
    );
}

#[test]
fn a_gate_that_is_not_armed_refuses_nothing() {
    assert_eq!(
        decide_scene_request(
            false,
            StoryRule::Hold,
            foreign(0x300, Some("next")),
            IN_THE_WORLD
        ),
        SceneRequestDecision::Pass
    );
    assert_eq!(
        decide_sub_phase(
            false,
            StoryRule::Hold,
            foreign(0x210, Some("next")),
            IN_THE_WORLD
        ),
        SubPhaseDecision::Pass
    );
}

#[test]
fn with_story_allowed_nothing_is_refused_and_foreign_requests_are_logged() {
    assert_eq!(
        decide_scene_request(
            true,
            StoryRule::Allow,
            foreign(0x300, Some("next")),
            IN_THE_WORLD
        ),
        SceneRequestDecision::PassAndLog
    );
    assert_eq!(
        decide_sub_phase(
            true,
            StoryRule::Allow,
            foreign(0x210, Some("next")),
            IN_THE_WORLD
        ),
        SubPhaseDecision::PassAndLog
    );
    assert_eq!(
        decide_scene_request(
            true,
            StoryRule::Allow,
            foreign(0x210, Some("made_up_step")),
            IN_THE_WORLD
        ),
        SceneRequestDecision::Pass
    );
}

#[test]
fn a_held_sub_phase_change_is_refused_unless_it_is_the_current_one() {
    assert_eq!(
        decide_sub_phase(
            true,
            StoryRule::Hold,
            foreign(0x210, Some("next")),
            IN_THE_WORLD
        ),
        SubPhaseDecision::Refuse
    );
    assert_eq!(
        decide_sub_phase(
            true,
            StoryRule::Hold,
            foreign(0x210, Some("made_up_step")),
            IN_THE_WORLD
        ),
        SubPhaseDecision::Pass
    );
}

#[test]
fn the_story_setting_reads_two_words_and_defaults_to_hold() {
    assert_eq!(StoryRule::from_text("hold"), Some(StoryRule::Hold));
    assert_eq!(StoryRule::from_text("allow"), Some(StoryRule::Allow));
    assert_eq!(StoryRule::from_text("other"), None);
    assert_eq!(StoryRule::default(), StoryRule::Hold);
    assert_eq!(StoryRule::Allow.label(), "allow");
}

#[test]
fn the_status_line_says_the_rule_and_whether_the_gate_is_armed() {
    assert!(story_status_line(StoryRule::Hold, true).starts_with("story: held"));
    assert!(story_status_line(StoryRule::Allow, true).starts_with("story: allowed"));
    assert!(story_status_line(StoryRule::Hold, false).ends_with("the gate is not armed"));
    assert!(story_status_line(StoryRule::Allow, false).ends_with("the gate is not armed"));
}

#[test]
fn the_log_lines_name_the_request() {
    assert_eq!(
        refused_scene_request_line(0x210, "made_up"),
        "story: refused scene request 0x210 made_up"
    );
    assert_eq!(
        refused_sub_phase_line(0x210, "made_up"),
        "story: refused sub-phase change 0x210 made_up"
    );
    assert!(foreign_scene_request_line(0x210, "made_up").contains("let through scene request"));
    assert!(foreign_sub_phase_line(0x210, "made_up").contains("let through sub-phase change"));
}

#[test]
fn the_way_back_to_the_title_does_not_open_the_sub_phase_gate() {
    assert_eq!(
        decide_sub_phase(
            true,
            StoryRule::Hold,
            foreign(0xF10, Some("made_up")),
            IN_THE_WORLD
        ),
        SubPhaseDecision::Refuse
    );
}

#[test]
fn the_scene_request_of_our_own_jump_gets_a_line_of_its_own() {
    assert_eq!(
        own_travel_scene_request_line(0x210, "made_up_step"),
        "story: let through the scene request of our own jump, 0x210 made_up_step"
    );
}

#[test]
fn a_request_is_our_jumps_only_while_the_window_is_open_and_the_transport_makes_it() {
    assert!(is_own_travel_request(true, Some(5)));
    assert!(!is_own_travel_request(false, Some(5)));
    for step in [-1, 0, 4, 6] {
        assert!(!is_own_travel_request(true, Some(step)));
    }
    assert!(!is_own_travel_request(true, None));
}

#[test]
fn our_own_jump_to_the_title_still_disarms_the_gate() {
    assert_eq!(
        decide_own_travel_request(0xF00),
        SceneRequestDecision::PassToTitleAndDisarm
    );
    assert_eq!(
        decide_own_travel_request(0xF10),
        SceneRequestDecision::PassToTitleAndDisarm
    );
    assert_eq!(decide_own_travel_request(0x210), SceneRequestDecision::Pass);
}

#[test]
fn the_games_defeat_flow_gets_its_scene_through_a_held_story() {
    for rule in [StoryRule::Hold, StoryRule::Allow] {
        assert_eq!(
            decide_scene_request(
                true,
                rule,
                of_the_defeat_flow(0x300, Some("restore_point")),
                IN_THE_WORLD
            ),
            SceneRequestDecision::PassForTheDefeatFlow
        );
        assert_eq!(
            decide_sub_phase(
                true,
                rule,
                of_the_defeat_flow(0x300, Some("restore_point")),
                IN_THE_WORLD
            ),
            SubPhaseDecision::PassForTheDefeatFlow
        );
    }
}

#[test]
fn a_defeat_flow_that_asks_for_the_title_still_disarms_the_gate() {
    assert_eq!(
        decide_scene_request(
            true,
            StoryRule::Hold,
            of_the_defeat_flow(0xF00, Some("START")),
            IN_THE_WORLD
        ),
        SceneRequestDecision::PassToTitleAndDisarm
    );
}

#[test]
fn the_defeat_flow_lines_name_the_phase() {
    assert_eq!(
        automata_game::story_gate::defeat_flow_scene_request_line(0x100, "made_up"),
        "story: let through the scene request of the game's defeat flow, 0x100 made_up"
    );
    assert_eq!(
        automata_game::story_gate::defeat_flow_sub_phase_line(0x100, "made_up"),
        "story: let through the sub-phase change of the game's defeat flow, 0x100 made_up"
    );
}

#[test]
fn the_follow_rule_reads_its_word_and_is_never_the_default() {
    assert_eq!(StoryRule::from_text("follow"), Some(StoryRule::Follow));
    assert_eq!(StoryRule::Follow.label(), "follow");
    assert_eq!(StoryRule::default(), StoryRule::Hold);
}

#[test]
fn the_follow_rule_leaves_a_foreign_request_to_the_follow_machine() {
    assert_eq!(
        decide_scene_request(
            true,
            StoryRule::Follow,
            foreign(0x300, Some("next")),
            IN_THE_WORLD
        ),
        SceneRequestDecision::LeaveToTheFollowRule
    );
    assert_eq!(
        decide_sub_phase(
            true,
            StoryRule::Follow,
            foreign(0x210, Some("other_step")),
            IN_THE_WORLD
        ),
        SubPhaseDecision::LeaveToTheFollowRule
    );
}

#[test]
fn the_follow_rule_still_passes_ours_the_current_phase_the_title_and_the_defeat_flow() {
    assert_eq!(
        decide_scene_request(true, StoryRule::Follow, ours(0x300), IN_THE_WORLD),
        SceneRequestDecision::Pass
    );
    assert_eq!(
        decide_scene_request(
            true,
            StoryRule::Follow,
            foreign(0x210, Some("made_up_step")),
            IN_THE_WORLD
        ),
        SceneRequestDecision::Pass
    );
    assert_eq!(
        decide_scene_request(
            true,
            StoryRule::Follow,
            foreign(0xF00, Some("START")),
            IN_THE_WORLD
        ),
        SceneRequestDecision::PassToTitleAndDisarm
    );
    assert_eq!(
        decide_scene_request(
            true,
            StoryRule::Follow,
            of_the_defeat_flow(0x300, Some("next")),
            IN_THE_WORLD
        ),
        SceneRequestDecision::PassForTheDefeatFlow
    );
}

#[test]
fn the_follow_status_line_says_the_host_leads() {
    assert!(story_status_line(StoryRule::Follow, true).contains("host leads"));
    assert!(story_status_line(StoryRule::Follow, false).contains("follow is set"));
}
