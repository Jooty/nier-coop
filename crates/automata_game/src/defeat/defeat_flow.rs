use crate::defeat::{
    CURRENT_POINT_STATE_WORD_THE_CONTINUE_WAITS_FOR, DEFEAT_FLOW_CHOOSING_STATE,
    DEFEAT_FLOW_CONTINUE_STEP, DEFEAT_FLOW_CONTINUE_WITHOUT_LEFT_BODY_STATE,
    DEFEAT_FLOW_ENDING_STATE, DEFEAT_FLOW_FADE_STEP, DEFEAT_FLOW_FIXED_PHASE_REQUEST_STATE,
    DEFEAT_FLOW_GAME_OVER_MENU_STATE, DEFEAT_FLOW_IDLE_STATE, DEFEAT_FLOW_LAST_STEP_BEFORE_THE_END,
    DEFEAT_FLOW_LEFT_BODY_SCREEN_STEPS, DEFEAT_FLOW_MENU_CLOSES_STEP,
    DEFEAT_FLOW_MENU_DROPS_THE_RECORD_STEP, DEFEAT_FLOW_MENU_FADES_IN_STEP,
    DEFEAT_FLOW_MENU_OPENS_STEP, DEFEAT_FLOW_MENU_SETS_ITS_COUNTER_STEP,
    DEFEAT_FLOW_MENU_WAITS_FOR_A_CHOICE_STEP, DEFEAT_FLOW_REQUESTED_STATE, DEFEAT_FLOW_TITLE_STATE,
    DEFEAT_FLOW_USUAL_STATE, DEFEAT_FLOW_WAIT_FOR_THE_CONTINUE_STEP, HERO_DEFEAT_STATES,
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

    /// The flow waits in its game-over menu for a choice (state 5, step 4).
    pub fn waits_in_the_game_over_menu(self) -> bool {
        self.state == DEFEAT_FLOW_GAME_OVER_MENU_STATE
            && self.step == DEFEAT_FLOW_MENU_WAITS_FOR_A_CHOICE_STEP
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
            DEFEAT_FLOW_USUAL_STATE => {
                format!("state 3 (back to the restore point), {}", usual_step(step))
            }
            DEFEAT_FLOW_CONTINUE_WITHOUT_LEFT_BODY_STATE => format!(
                "state 4 (back to the restore point, no screen for the body left behind), {}",
                usual_step(step)
            ),
            DEFEAT_FLOW_GAME_OVER_MENU_STATE => {
                format!("state 5 (the game-over menu), {}", menu_step(step))
            }
            DEFEAT_FLOW_FIXED_PHASE_REQUEST_STATE => format!(
                "state 6: the game asks for its fixed phase 0xF30 (step {step} picks the sub-phase)"
            ),
            DEFEAT_FLOW_TITLE_STATE => "state 7: the game asks for the title".to_owned(),
            DEFEAT_FLOW_ENDING_STATE => {
                "state 8: the flow ends, the game clears its input-off bits".to_owned()
            }
            _ => format!("state {state} (not a state of the game's flow), step {step}"),
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
        format!(
            "step {step}: waits for the current point's state word to read {CURRENT_POINT_STATE_WORD_THE_CONTINUE_WAITS_FOR:#X}"
        )
    } else if step == DEFEAT_FLOW_LAST_STEP_BEFORE_THE_END {
        format!("step {step}: one frame, then the flow ends")
    } else {
        format!("step {step}")
    }
}

fn menu_step(step: i32) -> String {
    match step {
        DEFEAT_FLOW_MENU_DROPS_THE_RECORD_STEP => {
            format!("step {step}: drops a record of the body left behind")
        }
        DEFEAT_FLOW_MENU_SETS_ITS_COUNTER_STEP => format!("step {step}: sets its frame counter"),
        DEFEAT_FLOW_MENU_FADES_IN_STEP => {
            format!("step {step}: counts down, waits for the fade, resets the menu")
        }
        DEFEAT_FLOW_MENU_OPENS_STEP => format!("step {step}: the menu opens"),
        DEFEAT_FLOW_MENU_WAITS_FOR_A_CHOICE_STEP => {
            format!("step {step}: the menu waits for a choice")
        }
        DEFEAT_FLOW_MENU_CLOSES_STEP => {
            format!("step {step}: the menu is done, the flow goes on by its cursor")
        }
        _ => format!("step {step}"),
    }
}
