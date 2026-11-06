use eframe::egui::{self, RichText, Ui};

use crate::app::pages::body_choice::draw_body_choice;
use crate::app::pages::lobby::{claim_mode, draw_lobby, other_page_note, session_open};
use crate::app::{Launcher, NetForms};
use crate::net::LOBBY_MODE_CONTINUE;

pub(super) fn draw_continue(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    if !claim_mode(launcher, forms, LOBBY_MODE_CONTINUE) {
        other_page_note(ui, launcher);
        return;
    }
    draw_slot_list(ui, launcher);
    draw_body_choice(ui, launcher, !session_open(forms));
    ui.separator();
    ui.label(
        RichText::new(
            "The host's choice decides: a guest without a save of their own starts a new game.",
        )
        .weak(),
    );
    draw_lobby(ui, launcher, forms);
    ui.separator();
    egui::CollapsingHeader::new("Copy and back up saves").show(ui, |ui| {
        draw_save_tools(ui, launcher);
    });
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
    if launcher.slots.is_empty() {
        ui.label(RichText::new("No co-op saves yet: start one on the New game page.").weak());
    }
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
        let can_copy = idle && !launcher.game_running && launcher.copy_from_slot.is_some();
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
        let can_restore = idle && !launcher.game_running && launcher.selected_backup.is_some();
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
    ui.label(
        RichText::new("Restoring first copies the current co-op saves to a before-restore backup.")
            .weak(),
    );
}
