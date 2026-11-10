use hudhook::imgui::{Condition, Ui};
use replicant_game::downed::{
    DownedState, LocalDownedLine, LocalDownedLineInputs, local_downed_line,
};

use crate::downed::{downed_status, request_go_down, request_revive_partner};

use crate::net;
use crate::overlay::enemy_sync_section::draw_enemy_sync;
use crate::overlay::minimap_dot::draw_minimap_tuning;
use crate::overlay::partner_marker::{
    marker_enabled, matrix_method, set_marker_enabled, set_matrix_method,
};
use crate::overlay::party_frame_tuning_panel::draw_party_frame_tuning;
use crate::overlay::stay_together_panel::draw_stay_together;
use crate::overlay::story_relay_section::draw_story_relay;
use crate::puppet::{ghost_enabled, ghost_status, set_ghost_enabled};
use crate::render_camera::{
    OFFSET_AUTO, camera_offset, camera_stats, set_camera_offset, set_tag_sync_uses_drawn_frame,
    stats_text, tag_sync_uses_drawn_frame,
};

#[derive(Debug, Default)]
pub(super) struct CoOpPanel;

impl CoOpPanel {
    pub(super) fn draw(&mut self, ui: &Ui, open: &mut bool) {
        if !*open {
            return;
        }
        ui.window("co-op")
            .opened(open)
            .position([372.0, 472.0], Condition::FirstUseEver)
            .size([420.0, 300.0], Condition::FirstUseEver)
            .build(|| {
                draw_party_frame_tuning(ui);
                draw_minimap_tuning(ui);
                ui.separator();
                let Some(net) = net::handle() else {
                    ui.text("launcher link: off (no nier-coop.toml with a link port)");
                    return;
                };
                let view = net.view();
                draw_ghost_controls(ui);
                draw_marker_controls(ui);
                ui.separator();
                draw_downed_controls(ui);
                ui.separator();
                draw_stay_together(ui, &view);
                ui.separator();
                draw_enemy_sync(ui);
                draw_story_relay(ui);
            });
    }
}

fn draw_downed_controls(ui: &Ui) {
    let status = downed_status();
    if !status.session_active {
        ui.text("downed and revive: off until a peer is connected");
        return;
    }
    ui.text(format!(
        "you: {}   partner: {}",
        state_label(status.local),
        state_label(status.partner)
    ));
    match local_downed_line(LocalDownedLineInputs {
        local: status.local,
        game_over: status.game_over,
        bleed_out_progress: status.bleed_out_progress,
        being_revived_progress: status.incoming_revive_progress,
    }) {
        LocalDownedLine::Nothing => {}
        LocalDownedLine::BleedingOut(progress) => ui.text(format!(
            "bleeding out: {:.0}% (partner must revive you)",
            progress * 100.0
        )),
        LocalDownedLine::WaitingForRevive => ui.text("Waiting for revive"),
        LocalDownedLine::BeingRevived(progress) => {
            ui.text(format!("being revived: {:.0}%", progress * 100.0));
        }
    }
    if status.game_over {
        ui.text("both downed: the game over flow is running");
    } else if status.partner == DownedState::Downed {
        ui.text(format!(
            "reviving partner: {:.0}% (stand close)",
            status.revive_progress * 100.0
        ));
    }
    ui.text(format!("Rewinds: {}", status.rewinds));
    if ui.button("revive partner (test)") {
        request_revive_partner();
    }
    ui.same_line();
    if ui.button("go down (test)") {
        request_go_down();
    }
}

fn state_label(state: DownedState) -> &'static str {
    match state {
        DownedState::Up => "up",
        DownedState::Downed => "DOWNED",
    }
}

fn draw_ghost_controls(ui: &Ui) {
    let mut enabled = ghost_enabled();
    if ui.checkbox("show the other player (ghost)", &mut enabled) {
        set_ghost_enabled(enabled);
    }
    let status = ghost_status();
    if status.spawned {
        ui.text("ghost: spawned");
    } else if let Some(reason) = status.wish.why_not() {
        ui.text(format!("ghost: not spawned, {reason}"));
    } else if let Some(error) = &status.spawn_error {
        ui.text(format!("ghost: not spawned, game refused ({error})"));
    } else {
        ui.text("ghost: spawning...");
    }
}

fn draw_marker_controls(ui: &Ui) {
    let mut enabled = marker_enabled();
    if ui.checkbox("partner name and edge arrow", &mut enabled) {
        set_marker_enabled(enabled);
    }
    draw_camera_offset_controls(ui);
    let mut matrix = matrix_method();
    if ui.checkbox(
        "marker: use camera matrix instead of view-projection",
        &mut matrix,
    ) {
        set_matrix_method(matrix);
    }
}

fn draw_camera_offset_controls(ui: &Ui) {
    let current = camera_offset();
    ui.text("camera frame offset (find where the tag sticks while turning):");
    if ui.radio_button_bool("auto", current == OFFSET_AUTO) {
        set_camera_offset(OFFSET_AUTO);
    }
    for offset in 0..=3_u8 {
        ui.same_line();
        if ui.radio_button_bool(format!("{offset}"), current == offset) {
            set_camera_offset(offset);
        }
    }
    let uses_drawn_frame = tag_sync_uses_drawn_frame();
    ui.text("Name tag sync:");
    ui.same_line();
    if ui.radio_button_bool("old##tag_sync", !uses_drawn_frame) {
        set_tag_sync_uses_drawn_frame(false);
    }
    ui.same_line();
    if ui.radio_button_bool("new##tag_sync", uses_drawn_frame) {
        set_tag_sync_uses_drawn_frame(true);
    }
    ui.text_wrapped(stats_text(&camera_stats()));
}
