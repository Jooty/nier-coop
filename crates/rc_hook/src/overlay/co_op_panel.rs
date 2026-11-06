use hudhook::imgui::{Condition, Ui};
use rc_game::PHASE_TABLE;

use crate::net::{self, NetCommand, NetShared, NetState};

#[derive(Debug, Default)]
pub(super) struct CoOpPanel {
    ticket_input: String,
}

impl CoOpPanel {
    pub(super) fn draw(&mut self, ui: &Ui, open: &mut bool) {
        if !*open {
            return;
        }
        ui.window("co-op")
            .opened(open)
            .position([372.0, 472.0], Condition::FirstUseEver)
            .size([420.0, 340.0], Condition::FirstUseEver)
            .build(|| {
                let Some(net) = net::handle() else {
                    ui.text("net thread not running");
                    return;
                };
                let view = net.view();
                ui.text(format!("state: {}", view.state.label()));
                self.draw_controls(ui, net, &view);
                ui.separator();
                draw_link_and_remote(ui, &view);
            });
    }

    fn draw_controls(&mut self, ui: &Ui, net: &net::NetThread, view: &NetShared) {
        match view.state {
            NetState::Hosting | NetState::Connected | NetState::Joining | NetState::Starting => {
                if let Some(ticket) = &view.ticket {
                    ui.text_wrapped(ticket);
                    if ui.button("copy ticket") {
                        ui.set_clipboard_text(ticket);
                    }
                    ui.same_line();
                }
                if ui.button("leave") {
                    net.send_command(NetCommand::Leave);
                }
            }
            NetState::Idle | NetState::Rejected(_) | NetState::Failed(_) => {
                if ui.button("host") {
                    net.send_command(NetCommand::Host);
                }
                ui.input_text("ticket", &mut self.ticket_input).build();
                if ui.button("paste") {
                    self.ticket_input = ui.clipboard_text().unwrap_or_default();
                }
                ui.same_line();
                if ui.button("join") {
                    net.send_command(NetCommand::Join(self.ticket_input.clone()));
                }
            }
        }
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
