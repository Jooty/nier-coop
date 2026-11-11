use crate::defeat::{
    DEFEAT_FLOW_CHOOSING_STATE, DEFEAT_FLOW_CONTINUE_STEP, DEFEAT_FLOW_ENDING_STATE,
    DEFEAT_FLOW_FADE_STEP, DEFEAT_FLOW_IDLE_STATE, DEFEAT_FLOW_LEFT_BODY_SCREEN_STEPS,
    DEFEAT_FLOW_REQUESTED_STATE, DEFEAT_FLOW_TITLE_STATE, DEFEAT_FLOW_USUAL_STATE,
    DEFEAT_FLOW_WAIT_FOR_THE_CONTINUE_STEP, HERO_DEFEAT_STATES,
};

/// The two numbers of the game's own flow after the hero's defeat: which part runs and its step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DefeatFlow {
    pub state: i32,
    pub step: i32,
}

impl DefeatFlow {
    pub fn is_active(self) -> bool {
        self.state != DEFEAT_FLOW_IDLE_STATE
    }

    pub fn asks_for_the_title(self) -> bool {
        self.state == DEFEAT_FLOW_TITLE_STATE
    }

    pub fn describe(self) -> String {
        let Self { state, step } = self;
        match state {
            DEFEAT_FLOW_IDLE_STATE => "idle".to_owned(),
            DEFEAT_FLOW_REQUESTED_STATE => {
                "state 1: a defeat was asked for, the game sets its input-off bits".to_owned()
            }
            DEFEAT_FLOW_CHOOSING_STATE => {
                "state 2: the game chooses what follows the defeat".to_owned()
            }
            DEFEAT_FLOW_USUAL_STATE => format!("state 3 (the usual flow), {}", usual_step(step)),
            DEFEAT_FLOW_TITLE_STATE => "state 7: the game asks for the title".to_owned(),
            DEFEAT_FLOW_ENDING_STATE => {
                "state 8: the flow ends, the game clears its input-off bits".to_owned()
            }
            _ => format!("state {state} (another kind of flow), step {step}"),
        }
    }
}

pub fn is_hero_defeat_state(state: i32) -> bool {
    HERO_DEFEAT_STATES.contains(&state)
}

fn usual_step(step: i32) -> String {
    if step == DEFEAT_FLOW_FADE_STEP {
        format!("step {step}: waits for the screen to fade")
    } else if DEFEAT_FLOW_LEFT_BODY_SCREEN_STEPS.contains(&step) {
        format!("step {step}: shows its screen about the body left behind")
    } else if step == DEFEAT_FLOW_CONTINUE_STEP {
        format!("step {step}: asks to continue from the restore point")
    } else if step == DEFEAT_FLOW_WAIT_FOR_THE_CONTINUE_STEP {
        format!("step {step}: waits for the continue to fade in")
    } else {
        format!("step {step}")
    }
}
