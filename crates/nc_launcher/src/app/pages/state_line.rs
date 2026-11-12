use eframe::egui::{self, RichText, Ui};
use nc_launcher_core::{StateLineInputs, session_state_line};

use crate::app::palette::{INK_MUTED, NOT_READY};
use crate::app::{Launcher, NetForms};

/// The connection's state or failure, drawn only when there is something to say.
pub(super) fn draw_state_line(ui: &mut Ui, launcher: &Launcher, forms: &NetForms) {
    let state = forms.net.status().state;
    let Some(line) = session_state_line(StateLineInputs {
        state: &state,
        own_ticket: launcher.link.ticket.is_some(),
        started: launcher.lobby_started,
        partner_in_play: launcher.link.partner_readiness().is_in_play(),
    }) else {
        return;
    };
    ui.horizontal(|ui| {
        if line.failed {
            ui.label(RichText::new(line.text).color(NOT_READY));
        } else {
            ui.add(egui::Spinner::new().color(INK_MUTED));
            ui.label(RichText::new(line.text).color(INK_MUTED));
        }
    });
}
