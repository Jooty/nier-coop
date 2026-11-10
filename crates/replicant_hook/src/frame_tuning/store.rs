use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use replicant_game::party_frame::PartyFrameTuning;

use crate::diagnostics::log_line;
use crate::frame_tuning::TUNING_FILE_NAME;

static TUNING: Mutex<PartyFrameTuning> = Mutex::new(PartyFrameTuning::new());

pub fn frame_tuning() -> PartyFrameTuning {
    *TUNING.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn set_frame_tuning(tuning: PartyFrameTuning) {
    *TUNING.lock().unwrap_or_else(PoisonError::into_inner) = tuning;
}

pub fn reset_frame_tuning() {
    set_frame_tuning(PartyFrameTuning::new());
}

pub fn save_frame_tuning() {
    let Some(path) = tuning_path() else {
        return;
    };
    if let Err(error) = std::fs::write(&path, frame_tuning().as_text()) {
        log_line(&format!("party frame: cannot save the layout ({error})"));
    }
}

pub(crate) fn load_frame_tuning() {
    let Some(path) = tuning_path() else {
        return;
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    set_frame_tuning(PartyFrameTuning::parse_text(&text));
    log_line("party frame: layout loaded from the tuning file");
}

fn tuning_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join(TUNING_FILE_NAME))
}
