use eframe::egui::{ScrollArea, Ui, UiBuilder};

use crate::app::pages::lobby::draw_lobby;
use crate::app::pages::lobby_layout::lobby_layout;
use crate::app::pages::lobby_options::draw_lobby_options;
use crate::app::pages::mode_menu::draw_mode_menu;
use crate::app::{Launcher, NetForms};

const MODES_TO_OPTIONS_GAP_PX: f32 = 26.0;

pub(super) fn draw_campaign_lobby(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    let layout = lobby_layout(ui.max_rect(), ui.ctx().content_rect());
    ui.scope_builder(UiBuilder::new().max_rect(layout.menu), |ui| {
        ScrollArea::vertical()
            .id_salt("lobby-menu-column")
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                draw_mode_menu(ui, launcher, forms);
                ui.add_space(MODES_TO_OPTIONS_GAP_PX);
                draw_lobby_options(ui, launcher, forms);
            });
    });
    ui.scope_builder(UiBuilder::new().max_rect(layout.main), |ui| {
        draw_lobby(ui, launcher, forms);
    });
}
