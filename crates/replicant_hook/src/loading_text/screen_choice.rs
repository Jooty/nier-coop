use std::sync::{Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use nt_hook::loading_text::{LoadingChoice, LoadingText, SessionPartner};
use replicant_game::loading_lines::DIARY_ENTRY_TEMPLATES;

use crate::diagnostics::log_line;
use crate::net::handle;

static LINES: Mutex<LoadingText> = Mutex::new(LoadingText::new(&DIARY_ENTRY_TEMPLATES));

pub(crate) fn set_loading_screen_lines(enabled: bool) {
    let mut lines = LINES.lock().unwrap_or_else(PoisonError::into_inner);
    lines.set_enabled(enabled);
    if let Ok(since_epoch) = SystemTime::now().duration_since(UNIX_EPOCH) {
        lines.reseed(since_epoch.as_nanos() as u64);
    }
    log_line(&format!(
        "loading screen lines: {} (loading_screen_lines in nier-coop.toml)",
        if enabled { "on" } else { "off" }
    ));
}

pub(super) fn choose_for_this_screen() -> Option<LoadingChoice> {
    let view = handle()?.view();
    let partner = SessionPartner::from_session(view.partner().is_some(), view.partner_name());
    let choice = LINES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .choose(partner)?;
    log_line(&format!(
        "loading screen lines: showing line {} of {} (partner name {} characters)",
        choice.line_index + 1,
        DIARY_ENTRY_TEMPLATES.len(),
        choice.partner_name_chars
    ));
    Some(choice)
}
