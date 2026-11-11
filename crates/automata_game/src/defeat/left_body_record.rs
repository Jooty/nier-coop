use crate::defeat::{
    DEFEAT_FLOW_CHOOSING_STATE, DEFEAT_FLOW_FADE_STEP, DEFEAT_FLOW_REQUESTED_STATE,
    DEFEAT_FLOW_USUAL_STATE, DefeatFlow,
};

/// In a linked session the record of the body left behind is dropped before the game's flow
/// reads it: with it the flow opens a screen that waits for a button, and sends the record out.
pub fn left_body_record_is_dropped(session_linked: bool, flow: DefeatFlow) -> bool {
    let before_the_flow_reads_it = match flow.state {
        DEFEAT_FLOW_REQUESTED_STATE | DEFEAT_FLOW_CHOOSING_STATE => true,
        DEFEAT_FLOW_USUAL_STATE => flow.step == DEFEAT_FLOW_FADE_STEP,
        _ => false,
    };
    session_linked && before_the_flow_reads_it
}
