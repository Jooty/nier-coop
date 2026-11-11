mod gate_decision;
mod story_rule;

pub use gate_decision::{
    CurrentPhase, SceneRequest, SceneRequestDecision, SubPhaseDecision, decide_own_travel_request,
    decide_scene_request, decide_sub_phase, defeat_flow_scene_request_line,
    defeat_flow_sub_phase_line, foreign_scene_request_line, foreign_sub_phase_line,
    is_own_travel_request, own_travel_scene_request_line, refused_scene_request_line,
    refused_sub_phase_line,
};
pub use story_rule::{StoryRule, story_status_line};

pub const RETURN_TO_TITLE_PHASE_ID: i32 = 0xF10;
