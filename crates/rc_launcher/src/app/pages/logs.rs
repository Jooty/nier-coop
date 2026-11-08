use eframe::egui::{self, RichText, Ui};

use crate::app::Launcher;

pub(super) fn draw_logs(ui: &mut Ui, launcher: &mut Launcher) {
    ui.horizontal(|ui| {
        if ui.button("Copy log").clicked() {
            ui.ctx().copy_text(launcher.log.join("\n"));
        }
        if ui.button("Export logs").clicked() {
            launcher.export_logs();
        }
        if launcher.log_upload_available()
            && ui
                .add_enabled(
                    !launcher.log_upload_busy(),
                    egui::Button::new("Send logs now"),
                )
                .clicked()
        {
            launcher.send_logs_now(ui.ctx());
        }
        if ui.button("Open logs folder").clicked() {
            launcher.open_logs_folder();
        }
        if let Some(note) = &launcher.export_note {
            ui.label(RichText::new(note).weak());
        }
    });
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            ui.set_max_width(ui.available_width());
            if launcher.log.is_empty() {
                ui.label(RichText::new("nothing logged yet").weak());
            }
            for line in &launcher.log {
                ui.add(egui::Label::new(RichText::new(line).monospace()).wrap());
            }
        });
}
