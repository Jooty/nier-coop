use crate::load_gate::loader_code::refers_to;
use crate::load_gate::{
    NEAR_CALL_OPCODE, SYSTEM_DATA_STAGE_WHILE_LOADING,
    SYSTEM_DIALOG_LINES_WHILE_THE_BOOT_SCREEN_IS_KEPT_AT_MOST, SYSTEM_DIALOG_OPEN_FUNCTION_START,
    TITLE_SYSTEM_DATA_STEP_CODE_CHECKED_LEN, TITLE_SYSTEM_DATA_STEP_DIALOG_OPEN_CALL_OFFSET,
    TITLE_SYSTEM_DATA_STEP_FUNCTION_START,
    TITLE_SYSTEM_DATA_STEP_STORES_THE_DIALOG_HANDLE_AND_TESTS_FOR_NONE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemDialogAnswer {
    LetItOpen,
    /// Answer as the game does when it cannot build the dialog: the title asks again next frame.
    HoldItBack,
}

/// What the hook does with the game's system dialogs while the boot's loading screen is kept, and
/// the log lines about them. The title's system data band is held back; every other dialog opens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BootScreenDialogs {
    held_back: u32,
    held_back_while_loading_logged: bool,
    held_back_otherwise_logged: bool,
    other_dialog_lines: u32,
}

impl BootScreenDialogs {
    /// One ask for a system dialog. `system_data_stage` is the game's system data stage, read for
    /// the log only (`None` when the read failed).
    pub fn ask(
        &mut self,
        screen_kept: bool,
        asked_by_the_title_system_data_step: bool,
        system_data_stage: Option<i32>,
    ) -> (SystemDialogAnswer, Option<String>) {
        if !screen_kept {
            return (SystemDialogAnswer::LetItOpen, None);
        }
        if !asked_by_the_title_system_data_step {
            let line = (self.other_dialog_lines
                < SYSTEM_DIALOG_LINES_WHILE_THE_BOOT_SCREEN_IS_KEPT_AT_MOST)
                .then(|| {
                    self.other_dialog_lines += 1;
                    format!(
                        "start: a system dialog was asked for outside the title's system data step while the boot's loading screen is kept (system data stage {}); it is shown",
                        stage_text(system_data_stage)
                    )
                });
            return (SystemDialogAnswer::LetItOpen, line);
        }
        self.held_back = self.held_back.saturating_add(1);
        let while_loading = system_data_stage == Some(SYSTEM_DATA_STAGE_WHILE_LOADING);
        let logged = if while_loading {
            &mut self.held_back_while_loading_logged
        } else {
            &mut self.held_back_otherwise_logged
        };
        let line = (!*logged).then(|| {
            *logged = true;
            let band = if while_loading {
                "loading band CORE_SYS_DIALOG_01"
            } else {
                "saving band CORE_SYS_DIALOG_02"
            };
            format!(
                "start: the title asked for its system data {band} (system data stage {}) under the boot's loading screen; held back, the system data load goes on",
                stage_text(system_data_stage)
            )
        });
        (SystemDialogAnswer::HoldItBack, line)
    }

    /// The line for the log when the boot's loading screen is let go.
    pub fn summary_when_the_keep_ends(&self) -> String {
        format!(
            "start: the title's system data band was asked for and held back {} times under the boot's loading screen; {} other system dialogs were shown",
            self.held_back, self.other_dialog_lines
        )
    }

    pub fn held_back(&self) -> u32 {
        self.held_back
    }
}

/// True when `code`, read at the start of the title's system data step, has the noted start and
/// opens its band through the system dialog opener at exactly `dialog_open_function`, keeping the
/// handle where the noted step keeps it. All addresses are offsets from the image base.
pub fn title_system_data_step_code_is_the_noted_one(
    code: &[u8],
    step_function: u32,
    dialog_open_function: u32,
) -> bool {
    let after_the_call =
        TITLE_SYSTEM_DATA_STEP_DIALOG_OPEN_CALL_OFFSET + NEAR_CALL_OPCODE.len() + 4;
    code.len() >= TITLE_SYSTEM_DATA_STEP_CODE_CHECKED_LEN
        && code.starts_with(&TITLE_SYSTEM_DATA_STEP_FUNCTION_START)
        && refers_to(
            code,
            step_function,
            TITLE_SYSTEM_DATA_STEP_DIALOG_OPEN_CALL_OFFSET,
            &NEAR_CALL_OPCODE,
            0,
        ) == Some(dialog_open_function)
        && code.get(after_the_call..TITLE_SYSTEM_DATA_STEP_CODE_CHECKED_LEN)
            == Some(&TITLE_SYSTEM_DATA_STEP_STORES_THE_DIALOG_HANDLE_AND_TESTS_FOR_NONE[..])
}

pub fn system_dialog_open_code_is_the_noted_one(code: &[u8]) -> bool {
    code.starts_with(&SYSTEM_DIALOG_OPEN_FUNCTION_START)
}

fn stage_text(stage: Option<i32>) -> String {
    stage.map_or_else(|| "unread".to_owned(), |stage| stage.to_string())
}
