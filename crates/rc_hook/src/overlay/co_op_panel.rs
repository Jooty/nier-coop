use hudhook::imgui::{Condition, Ui};
use rc_game::PHASE_TABLE;
use rc_game::downed::DownedState;

use crate::downed::{downed_status, request_go_down, request_revive_partner};

use crate::net::{self, NetShared, RemotePlayer};
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
                    ui.text("launcher link: off (no replicant-coop.toml with a link port)");
                    return;
                };
                let view = net.view();
                if let Some(port) = view.link_port {
                    ui.text(format!("launcher link: 127.0.0.1:{port}"));
                }
                ui.text(format!("state: {}", view.state.label()));
                ui.text("host and join live in the launcher");
                ui.separator();
                draw_link_and_remote(ui, &view);
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

fn draw_link_and_remote(ui: &Ui, view: &NetShared) {
    ui.text(format!(
        "peer: {}",
        view.peer_name.as_deref().unwrap_or("none")
    ));
    if view.partner_joining() {
        ui.text(if view.partner.is_game_closed() {
            rc_game::readiness::PARTNER_GAME_CLOSED_LABEL
        } else {
            rc_game::readiness::joining_label(view.partner.readiness())
        });
    }
    match view.rtt {
        Some(rtt) => ui.text(format!("ping: {:.0} ms", rtt.as_secs_f32() * 1000.0)),
        None => ui.text("ping: unknown"),
    }
    ui.text(match view.direct_path {
        Some(true) => "path: direct",
        Some(false) => "path: relay",
        None => "path: unknown",
    });
    ui.separator();
    ui.text(format!(
        "your phase: {}",
        view.local
            .as_ref()
            .map_or("unknown", |local| phase_name(local.phase))
    ));
    let Some(remote) = &view.remote else {
        ui.text("peer phase: no state received yet");
        draw_ghost_controls(ui);
        draw_marker_controls(ui);
        return;
    };
    ui.text(format!("peer phase: {}", phase_name(remote.state.phase)));
    ui.text(format!("last state: {} ms ago", remote.age().as_millis()));
    ui.text(format!(
        "peer hp: {} / {}",
        remote.state.hp, remote.state.max_hp
    ));
    ui.text(distance_text(view, remote));
    draw_ghost_controls(ui);
    draw_marker_controls(ui);
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
    if status.local == DownedState::Downed && !status.game_over {
        ui.text(format!(
            "bleeding out: {:.0}% (partner must revive you)",
            status.bleed_out_progress * 100.0
        ));
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

fn phase_name(phase: u16) -> &'static str {
    PHASE_TABLE
        .get(usize::from(phase))
        .map_or("unknown phase", |entry| entry.name)
}

fn distance_text(view: &NetShared, remote: &RemotePlayer) -> String {
    let Some(local) = &view.local else {
        return "distance: unknown".to_owned();
    };
    if local.phase != remote.state.phase {
        return "distance: not in the same phase".to_owned();
    }
    match distance(local.position, remote.state.position) {
        Some(units) => format!("distance: {units:.1}"),
        None => "distance: unknown".to_owned(),
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

fn distance(from: [f32; 3], to: [f32; 3]) -> Option<f32> {
    let squared: f32 = from.iter().zip(to).map(|(a, b)| (a - b) * (a - b)).sum();
    squared.is_finite().then(|| squared.sqrt())
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
