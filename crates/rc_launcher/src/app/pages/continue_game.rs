use eframe::egui::{self, RichText, Ui};

use crate::app::pages::lobby::{claim_mode, draw_lobby, other_page_note};
use crate::app::{Launcher, NetForms};
use crate::lobby::body_label;
use crate::net::LOBBY_MODE_CONTINUE;
use crate::saves::slot_body;

pub(super) fn draw_continue(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    if !claim_mode(launcher, forms, LOBBY_MODE_CONTINUE) {
        other_page_note(ui, launcher);
        return;
    }
    draw_slot_list(ui, launcher);
    draw_slot_character(ui, launcher);
    ui.separator();
    draw_lobby(ui, launcher, forms);
    ui.separator();
    egui::CollapsingHeader::new("Copy and back up saves").show(ui, |ui| {
        draw_save_tools(ui, launcher);
    });
}

fn draw_slot_character(ui: &mut Ui, launcher: &Launcher) {
    let Some(slot) = launcher.selected_slot else {
        return;
    };
    let body = slot_body(&launcher.slot_bodies, slot);
    ui.label(format!("Your character in this save: {}", body_label(body)));
}

fn draw_slot_list(ui: &mut Ui, launcher: &mut Launcher) {
    ui.label("Co-op saves (pick the one to continue)");
    let mut picked = launcher.selected_slot;
    egui::Grid::new("slots")
        .num_columns(5)
        .striped(true)
        .show(ui, |ui| {
            for slot in &launcher.slots {
                ui.add_enabled_ui(slot.used, |ui| {
                    ui.selectable_value(
                        &mut picked,
                        Some(slot.index),
                        format!("Slot {}", slot.index),
                    );
                });
                if slot.used {
                    ui.label(&slot.name);
                    ui.label(format!("level {}", slot.level));
                    ui.label(slot.play_time_text());
                    ui.label(&slot.phase);
                } else {
                    ui.label(RichText::new("empty").weak());
                }
                ui.end_row();
            }
        });
    launcher.selected_slot = picked;
}

fn draw_save_tools(ui: &mut Ui, launcher: &mut Launcher) {
    ui.label("Copy a slot from my normal saves (never changes the normal file)");
    let idle = !launcher.busy();
    ui.horizontal(|ui| {
        for slot in launcher.normal_slots.clone() {
            if slot.used {
                ui.selectable_value(
                    &mut launcher.copy_from_slot,
                    Some(slot.index),
                    format!("{} (level {})", slot.name, slot.level),
                );
            }
        }
        let can_copy = idle && !launcher.any_game_running && launcher.copy_from_slot.is_some();
        if ui
            .add_enabled(can_copy, egui::Button::new("Copy into co-op"))
            .clicked()
        {
            launcher.copy_normal_slot_into_coop();
        }
    });
    ui.separator();
    ui.label("Co-op save backups");
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                launcher.game.is_some() && idle,
                egui::Button::new("Back up now"),
            )
            .clicked()
        {
            launcher.backup_coop_saves();
        }
        let can_restore = idle && !launcher.any_game_running && launcher.selected_backup.is_some();
        if ui
            .add_enabled(can_restore, egui::Button::new("Restore selected"))
            .clicked()
        {
            launcher.restore_selected_backup();
        }
    });
    egui::ScrollArea::vertical()
        .max_height(150.0)
        .show(ui, |ui| {
            for stamp in launcher.backups.clone() {
                ui.selectable_value(&mut launcher.selected_backup, Some(stamp.clone()), stamp);
            }
        });
}
