use eframe::egui::{self, Ui};
use rc_launcher_core::{MAX_ENEMY_DAMAGE_PERCENT, MAX_ENEMY_HEALTH_PERCENT};

use crate::app::{Launcher, NetForms};
use crate::lobby::host_options_editable;

pub(super) fn draw_host_options(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    if !host_options_editable(launcher.link.guest) {
        let options = launcher.host_options_seen.unwrap_or_default();
        ui.add_enabled(
            false,
            egui::Checkbox::new(&mut { options.show_names }, "Show names over players"),
        );
        ui.add_enabled(
            false,
            egui::Checkbox::new(&mut { options.show_party_frame }, "Show party frame"),
        );
        ui.add_enabled(
            false,
            egui::Checkbox::new(&mut { options.show_distance }, "Show distance"),
        );
        ui.add_enabled(
            false,
            egui::Checkbox::new(&mut { options.show_ping }, "Show ping"),
        );
        ui.add_enabled(
            false,
            egui::Slider::new(
                &mut { options.enemy_health_percent },
                0..=MAX_ENEMY_HEALTH_PERCENT,
            )
            .suffix("%")
            .text("Enemy health per extra player"),
        );
        ui.add_enabled(
            false,
            egui::Slider::new(
                &mut { options.enemy_damage_percent },
                0..=MAX_ENEMY_DAMAGE_PERCENT,
            )
            .suffix("%")
            .text("Enemy damage per extra player"),
        );
        return;
    }
    let mut changed = false;
    changed |= ui
        .checkbox(&mut launcher.config.show_names, "Show names over players")
        .on_hover_text("Host only: everyone in the session sees the host's choice")
        .changed();
    changed |= ui
        .checkbox(&mut launcher.config.show_party_frame, "Show party frame")
        .on_hover_text("Host only: everyone in the session sees the host's choice")
        .changed();
    changed |= ui
        .checkbox(&mut launcher.config.show_distance, "Show distance")
        .on_hover_text("Host only: adds the partner's distance to the name tag")
        .changed();
    changed |= ui
        .checkbox(&mut launcher.config.show_ping, "Show ping")
        .on_hover_text("Host only: adds the connection's ping to the name tag")
        .changed();
    changed |= ui
        .add(
            egui::Slider::new(
                &mut launcher.config.enemy_health_percent,
                0..=MAX_ENEMY_HEALTH_PERCENT,
            )
            .suffix("%")
            .text("Enemy health per extra player"),
        )
        .on_hover_text("Host only: enemies have this much more health per extra player")
        .changed();
    changed |= ui
        .add(
            egui::Slider::new(
                &mut launcher.config.enemy_damage_percent,
                0..=MAX_ENEMY_DAMAGE_PERCENT,
            )
            .suffix("%")
            .text("Enemy damage per extra player"),
        )
        .on_hover_text("Host only: enemy hits on either player hurt this much more")
        .changed();
    if changed {
        launcher.apply_settings();
        forms.session_options = launcher.config.session_options();
        forms.push_session_options();
    }
}
