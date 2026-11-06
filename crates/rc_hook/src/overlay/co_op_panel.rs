use hudhook::imgui::{Condition, Ui};
use rc_game::PHASE_TABLE;

use crate::net::{self, NetShared};

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
        Some(rtt) => ui.text(format!("rtt: {:.1} ms", rtt.as_secs_f32() * 1000.0)),
        None => ui.text("rtt: unknown"),
    }
    ui.text(match view.direct_path {
        Some(true) => "path: direct",
        Some(false) => "path: relay",
        None => "path: unknown",
    });
    let Some(remote) = &view.remote else {
        ui.text("remote player: no state yet");
        return;
    };
    let phase = PHASE_TABLE
        .get(usize::from(remote.state.phase))
        .map_or("unknown phase", |entry| entry.name);
    let [x, y, z] = remote.state.position;
    ui.text(format!("remote phase: {phase}"));
    ui.text(format!("remote position: {x:.2} {y:.2} {z:.2}"));
    ui.text(format!(
        "remote hp: {} / {}",
        remote.state.hp, remote.state.max_hp
    ));
    ui.text(format!("last state: {} ms ago", remote.age().as_millis()));
}
