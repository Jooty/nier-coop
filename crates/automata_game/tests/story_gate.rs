use automata_game::story_gate::{
    GateDecision, decide, refused_scene_request_line, refused_sub_phase_line,
};

#[test]
fn an_armed_gate_refuses_what_is_not_ours() {
    assert_eq!(decide(true, false), GateDecision::Refuse);
}

#[test]
fn our_own_calls_always_pass() {
    assert_eq!(decide(true, true), GateDecision::PassThrough);
    assert_eq!(decide(false, true), GateDecision::PassThrough);
}

#[test]
fn a_gate_that_is_not_armed_refuses_nothing() {
    assert_eq!(decide(false, false), GateDecision::PassThrough);
}

#[test]
fn the_refusal_lines_name_the_request() {
    assert_eq!(
        refused_scene_request_line(0x210, "made_up"),
        "story: refused scene request 0x210 made_up"
    );
    assert!(refused_sub_phase_line(0x5).starts_with("story: refused sub-phase change 0x5"));
}
