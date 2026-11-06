use hudhook::imgui::Ui;
use rc_proto::EnemyId;

use crate::enemies::{
    EnemySyncRole, EnemySyncStatus, enemy_id_text, enemy_sync_status, set_snap_enabled,
    snap_enabled,
};

const MAX_IDS_LISTED: usize = 8;

pub(super) fn draw_enemy_sync(ui: &Ui) {
    ui.separator();
    ui.text("shared enemies (test readout)");
    let mut snap = snap_enabled();
    if ui.checkbox("guest: snap my enemies to the host's", &mut snap) {
        set_snap_enabled(snap);
    }
    let status = enemy_sync_status();
    match status.role {
        Some(EnemySyncRole::Host) => draw_host(ui, &status),
        Some(EnemySyncRole::Guest) => draw_guest(ui, &status),
        None => {}
    }
    if let Some(reason) = status.idle_reason {
        ui.text(format!("enemy sync: {reason}"));
    }
    for event in &status.recent_events {
        ui.text_disabled(event);
    }
}

fn draw_host(ui: &Ui, status: &EnemySyncStatus) {
    ui.text(format!(
        "you are the host: {} enemies here, sent in {} parts",
        status.local_count, status.sent_parts
    ));
}

fn draw_guest(ui: &Ui, status: &EnemySyncStatus) {
    let host = status
        .host_total
        .map_or_else(|| "unknown".to_owned(), |total| total.to_string());
    ui.text(format!(
        "enemies: {} here, {host} at the host ({} received)",
        status.local_count, status.mirrored
    ));
    ui.text(format!("matched: {}", status.matched));
    ui.text(format!("only here: {}", id_list(&status.unmatched_local)));
    ui.text(format!(
        "only at the host: {}",
        id_list(&status.unmatched_remote)
    ));
    match status.worst_error {
        Some((id, error)) => ui.text(format!(
            "worst position error: {error:.2} ({})",
            enemy_id_text(id)
        )),
        None => ui.text("worst position error: none matched"),
    }
    if snap_enabled() {
        ui.text(format!(
            "snapped: {} this frame, {} failed",
            status.snapped, status.snap_failures
        ));
    }
}

fn id_list(ids: &[EnemyId]) -> String {
    if ids.is_empty() {
        return "none".to_owned();
    }
    let mut listed: Vec<String> = ids
        .iter()
        .take(MAX_IDS_LISTED)
        .map(|id| enemy_id_text(*id))
        .collect();
    if ids.len() > MAX_IDS_LISTED {
        listed.push(format!("and {} more", ids.len() - MAX_IDS_LISTED));
    }
    listed.join(", ")
}
