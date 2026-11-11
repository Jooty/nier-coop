use crate::load_gate::{
    BOOT_SCREEN_KEPT_PAST_THE_BOOT_LOAD_AT_MOST_MS, LOAD_DONE_REQUESTED,
    LOADING_SCREEN_NOT_ANSWERED, SCENE_LOADER_LAST_STAGE, SCENE_STATE_WHILE_A_LOAD_RUNS,
};

/// How far the hook on the scene loader is with letting the boot's load end behind its screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BootLoadPass {
    #[default]
    NotYet,
    /// The loader was given the loading screen's answer in the screen's place and has yet to end.
    AnswerLent,
    Passed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootLoadEndAction {
    Nothing,
    /// Take the request for the load's end back, so the screen stays, and answer in its place.
    PassTheLoadKeepTheScreen,
    TakeTheAnswerBack,
}

/// What the scene loader left behind after one of its steps; `None` is a failed read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LoaderReading {
    pub stage: Option<i32>,
    pub load_done_request: Option<i32>,
    pub loading_screen_answer: Option<i32>,
    pub loader_finished: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootScreenState {
    NotKept,
    Kept,
    Over,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootScreenInputs {
    pub now_ms: u64,
    pub boot_load_passed: bool,
    /// The start has asked the game for the scene it starts in.
    pub next_load_asked: bool,
    pub start_is_over: bool,
    pub scene_state: i32,
}

/// Keeps the loading screen of the game's boot up over the skipped title, so the load of the start
/// goes on under the same screen and the player sees one loading screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootScreenKeep {
    state: BootScreenState,
    boot_load_passed_at_ms: Option<u64>,
}

impl LoaderReading {
    /// True right after the loader's fourth stage asked the loading screen to go away.
    pub fn has_just_asked_for_the_end_of_the_load(&self) -> bool {
        !self.loader_finished
            && self.stage == Some(SCENE_LOADER_LAST_STAGE)
            && self.load_done_request == Some(LOAD_DONE_REQUESTED)
            && self.loading_screen_answer == Some(LOADING_SCREEN_NOT_ANSWERED)
    }
}

impl BootScreenKeep {
    pub fn new(
        setting_on: bool,
        start_skips_the_title: bool,
        hooks_installed: bool,
    ) -> (Self, Vec<String>) {
        let off = if !start_skips_the_title {
            return (Self::unused(), Vec::new());
        } else if !setting_on {
            Some("the setting one_loading_screen is off")
        } else if !hooks_installed {
            Some("the game's loading screen could not be hooked")
        } else {
            None
        };
        match off {
            Some(why) => (
                Self::unused(),
                vec![format!(
                    "start: {why}, the game shows a loading screen before the title and another after it"
                )],
            ),
            None => (
                Self {
                    state: BootScreenState::Kept,
                    boot_load_passed_at_ms: None,
                },
                vec![
                    "start: one loading screen: the boot's loading screen is kept over the skipped title until the load of the start is done"
                        .to_owned(),
                ],
            ),
        }
    }

    pub fn unused() -> Self {
        Self {
            state: BootScreenState::NotKept,
            boot_load_passed_at_ms: None,
        }
    }

    pub fn state(&self) -> BootScreenState {
        self.state
    }

    pub fn wants_the_screen_kept(&self) -> bool {
        self.state == BootScreenState::Kept
    }

    pub fn step(&mut self, inputs: &BootScreenInputs) -> Vec<String> {
        if self.state != BootScreenState::Kept {
            return Vec::new();
        }
        if inputs.boot_load_passed && self.boot_load_passed_at_ms.is_none() {
            self.boot_load_passed_at_ms = Some(inputs.now_ms);
        }
        let kept_ms = self
            .boot_load_passed_at_ms
            .map_or(0, |passed| inputs.now_ms.saturating_sub(passed));
        let over = if inputs.next_load_asked && inputs.scene_state == SCENE_STATE_WHILE_A_LOAD_RUNS
        {
            Some(
                "start: the load of the start runs under the boot's loading screen, which ends with it"
                    .to_owned(),
            )
        } else if inputs.start_is_over {
            Some("start: no load follows the title, the loading screen is let go".to_owned())
        } else if kept_ms >= BOOT_SCREEN_KEPT_PAST_THE_BOOT_LOAD_AT_MOST_MS {
            Some(format!(
                "start: the loading screen was kept for {kept_ms} ms after the boot's load and no load followed, it is let go"
            ))
        } else {
            None
        };
        match over {
            Some(line) => {
                self.state = BootScreenState::Over;
                vec![line]
            }
            None => Vec::new(),
        }
    }
}

/// What the hook does after one step of the scene loader so that the boot's load ends while its
/// loading screen stays. Only the first load that ends while the screen is wanted kept is passed.
pub fn boot_load_end_action(
    pass: BootLoadPass,
    screen_wanted_kept: bool,
    reading: &LoaderReading,
) -> BootLoadEndAction {
    match pass {
        BootLoadPass::NotYet
            if screen_wanted_kept && reading.has_just_asked_for_the_end_of_the_load() =>
        {
            BootLoadEndAction::PassTheLoadKeepTheScreen
        }
        // The answer is the screen's to give at the end of the next load: it must not stay.
        BootLoadPass::AnswerLent
            if reading.loader_finished || reading.stage != Some(SCENE_LOADER_LAST_STAGE) =>
        {
            BootLoadEndAction::TakeTheAnswerBack
        }
        _ => BootLoadEndAction::Nothing,
    }
}
