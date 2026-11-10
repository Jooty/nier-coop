use std::io::{Read, Seek, SeekFrom};

use hudhook::imgui::Ui;
use nc_ui_style::{WARN, rgba_to_f32};
use replicant_game::fix_buttons::{
    ButtonFacts, COPIED_LOG_LINES, FixButton, FixButtonState, LOG_TAIL_READ_BYTES, tail_lines,
};
use replicant_game::stay_together::{FollowRole, distance_between};

use crate::diagnostics::{log_line, log_path};
use crate::net::NetShared;
use crate::puppet::request_ghost_respawn;
use crate::stay_together::{leash, request_go_to_partner, request_rejoin_host_area, stay_status};
use crate::story_progress::request_story_resync;

pub(super) fn draw_fix_buttons(ui: &Ui, view: &NetShared) {
    ui.separator();
    ui.text("fix buttons");
    let facts = ButtonFacts {
        role: FollowRole::from_slot(view.local_slot),
        connected: view.partner_in_play(),
    };
    draw_far_warning(ui, view);
    if press(ui, "go to partner", FixButton::GoToPartner, facts) {
        request_go_to_partner();
    }
    show_status(ui, &stay_status().go_to_partner);
    if press(ui, "resync story", FixButton::ResyncStory, facts) {
        request_story_resync();
    }
    if press(
        ui,
        "rejoin the host's area",
        FixButton::RejoinHostArea,
        facts,
    ) {
        request_rejoin_host_area();
    }
    show_status(ui, &stay_status().rejoin);
    if press(ui, "respawn partner", FixButton::RespawnPartner, facts) {
        request_ghost_respawn();
    }
    if press(ui, "copy logs", FixButton::CopyLogs, facts) {
        copy_logs(ui);
    }
    ui.text_wrapped(format!("log file: {}", log_path()));
}

fn press(ui: &Ui, label: &str, button: FixButton, facts: ButtonFacts) -> bool {
    let state = button.state(facts);
    let pressed = {
        let _disabled = ui.begin_disabled(state != FixButtonState::Enabled);
        ui.button(label)
    };
    if let FixButtonState::Disabled(reason) = state
        && ui.is_item_hovered_with_flags(hudhook::imgui::ItemHoveredFlags::ALLOW_WHEN_DISABLED)
    {
        ui.tooltip_text(reason);
    }
    if pressed {
        log_line(&format!("fix button pressed: {}", button.log_id()));
    }
    pressed
}

fn show_status(ui: &Ui, text: &str) {
    if !text.is_empty() {
        ui.text_wrapped(text);
    }
}

fn draw_far_warning(ui: &Ui, view: &NetShared) {
    let far = match (&view.local, &view.remote) {
        (Some(local), Some(remote)) if local.phase == remote.state.phase => {
            leash().is_exceeded_by(distance_between(local.position, remote.state.position))
        }
        _ => false,
    };
    if far {
        ui.text_colored(rgba_to_f32(WARN), "you are far from the other player");
    }
}

fn copy_logs(ui: &Ui) {
    let path = log_path();
    let tail = read_log_tail(path).unwrap_or_default();
    let text = format!("log file: {path}\n{}", tail_lines(&tail, COPIED_LOG_LINES));
    ui.set_clipboard_text(text);
    log_line(&format!(
        "fix button copy_logs: copied the last {COPIED_LOG_LINES} lines of {path}"
    ));
}

fn read_log_tail(path: &str) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(length.saturating_sub(LOG_TAIL_READ_BYTES)))
        .ok()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}
