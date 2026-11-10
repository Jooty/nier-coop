use crate::start::TITLE_PHASE_ID;
use crate::story_gate::{RETURN_TO_TITLE_PHASE_ID, StoryRule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneRequest<'a> {
    pub ours: bool,
    pub phase_id: i32,
    pub phase_name: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentPhase<'a> {
    pub id: i32,
    pub name: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneRequestDecision {
    Pass,
    PassToTitleAndDisarm,
    PassAndLog,
    Refuse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubPhaseDecision {
    Pass,
    PassAndLog,
    Refuse,
}

pub fn decide_scene_request(
    armed: bool,
    rule: StoryRule,
    request: SceneRequest<'_>,
    current: CurrentPhase<'_>,
) -> SceneRequestDecision {
    if !armed || request.ours {
        return SceneRequestDecision::Pass;
    }
    if request.phase_id == TITLE_PHASE_ID || request.phase_id == RETURN_TO_TITLE_PHASE_ID {
        return SceneRequestDecision::PassToTitleAndDisarm;
    }
    if is_current_phase(request.phase_id, request.phase_name, current) {
        return SceneRequestDecision::Pass;
    }
    match rule {
        StoryRule::Hold => SceneRequestDecision::Refuse,
        StoryRule::Allow => SceneRequestDecision::PassAndLog,
    }
}

pub fn decide_sub_phase(
    armed: bool,
    rule: StoryRule,
    request: SceneRequest<'_>,
    current: CurrentPhase<'_>,
) -> SubPhaseDecision {
    if !armed || request.ours {
        return SubPhaseDecision::Pass;
    }
    if is_current_phase(request.phase_id, request.phase_name, current) {
        return SubPhaseDecision::Pass;
    }
    match rule {
        StoryRule::Hold => SubPhaseDecision::Refuse,
        StoryRule::Allow => SubPhaseDecision::PassAndLog,
    }
}

pub fn refused_scene_request_line(phase_id: i32, phase_name: &str) -> String {
    format!("story: refused scene request {phase_id:#X} {phase_name}")
}

pub fn foreign_scene_request_line(phase_id: i32, phase_name: &str) -> String {
    format!("story: let through scene request {phase_id:#X} {phase_name} (story = allow)")
}

pub fn own_travel_scene_request_line(phase_id: i32, phase_name: &str) -> String {
    format!("story: let through the scene request of our own jump, {phase_id:#X} {phase_name}")
}

pub fn refused_sub_phase_line(phase_id: i32, phase_name: &str) -> String {
    format!("story: refused sub-phase change {phase_id:#X} {phase_name}")
}

pub fn foreign_sub_phase_line(phase_id: i32, phase_name: &str) -> String {
    format!("story: let through sub-phase change {phase_id:#X} {phase_name} (story = allow)")
}

fn is_current_phase(phase_id: i32, phase_name: Option<&str>, current: CurrentPhase<'_>) -> bool {
    // A request without a name takes the phase's first sub-phase, which may not be the current one.
    phase_id == current.id
        && phase_name.is_some_and(|name| !name.is_empty())
        && phase_name == current.name
}
