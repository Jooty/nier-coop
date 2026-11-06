use hudhook::imgui::{Condition, Ui};
use rc_game::PHASE_TABLE;

use crate::net::{self, NetShared, RemotePlayer};
use crate::puppet::{ghost_enabled, ghost_status, set_ghost_enabled};

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
                let Some(net) = net::handle() else {
                    ui.text("launcher link: off (no replicant-coop.toml with a link port)");
                    return;
                };
                let view = net.view();
                if let Some(port) = view.link_port {
                    ui.text(format!("launcher link: 127.0.0.1:{port}"));
                }
                ui.text(format!("state: {}", view.state.label()));
                ui.text("host, join and the password live in the launcher");
                ui.separator();
                draw_link_and_remote(ui, &view);
            });
    }
}

fn draw_link_and_remote(ui: &Ui, view: &NetShared) {
    ui.text(format!(
        "peer: {}",
        view.peer_name.as_deref().unwrap_or("none")
    ));
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

fn distance(from: [f32; 3], to: [f32; 3]) -> Option<f32> {
    let squared: f32 = from.iter().zip(to).map(|(a, b)| (a - b) * (a - b)).sum();
    squared.is_finite().then(|| squared.sqrt())
}
