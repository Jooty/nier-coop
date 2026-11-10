#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDecision {
    PassThrough,
    Refuse,
}

pub fn decide(armed: bool, ours: bool) -> GateDecision {
    if armed && !ours {
        GateDecision::Refuse
    } else {
        GateDecision::PassThrough
    }
}

pub fn refused_scene_request_line(phase_id: i32, phase_name: &str) -> String {
    format!("story: refused scene request {phase_id:#X} {phase_name}")
}

pub fn refused_sub_phase_line(second_argument: u64) -> String {
    format!("story: refused sub-phase change {second_argument:#X} (argument layout is a guess)")
}
