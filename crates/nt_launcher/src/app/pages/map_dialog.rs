use eframe::egui::{Align, Layout, Rect, Ui, UiBuilder, pos2};
use nt_launcher_core::{current_map, map_pick_offered, rules_with_map};

use crate::app::pages::dialog_frame::show_dialog;
use crate::app::pages::map_banner::paint_map_banner;
use crate::app::parts::{MenuValue, command_button, menu_row, section_heading};
use crate::app::{Launcher, NetForms};

const DIALOG_SIZE_PX: (f32, f32) = (840.0, 440.0);
const DIALOG_PADDING_PX: f32 = 28.0;
const HEADING_HEIGHT_PX: f32 = 30.0;
const HEADING_TO_BODY_PX: f32 = 14.0;
const LIST_WIDTH_PX: f32 = 240.0;
const LIST_TO_BANNER_GAP_PX: f32 = 28.0;
const BANNER_TO_COMMAND_GAP_PX: f32 = 20.0;
const BANNER_HEIGHT_PER_WIDTH: f32 = 9.0 / 16.0;
const DIALOG_NAME_PX: f32 = 32.0;
const COMMAND_SIZE_PX: (f32, f32) = (176.0, 48.0);
const DIALOG_LAYER: &str = "map-dialog";
const MAP_HEADING: &str = "Map";
const DONE_LABEL: &str = "Done";

/// The host's map choice: every map on the left, the selected one's banner on the right. A pick reaches the
/// guests at once.
pub(super) fn draw_map_dialog(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    let rules = launcher.link.rules;
    if !map_pick_offered(launcher.profile, rules, launcher.link.guest) {
        forms.map_dialog_open = false;
        return;
    }
    let mut done = false;
    let close = show_dialog(ui, DIALOG_SIZE_PX, DIALOG_LAYER, |ui, dialog| {
        done = draw_dialog(ui, dialog, launcher, forms);
    });
    if close || done {
        forms.map_dialog_open = false;
    }
}

fn draw_dialog(ui: &mut Ui, dialog: Rect, launcher: &mut Launcher, forms: &mut NetForms) -> bool {
    let profile = launcher.profile;
    let inner = dialog.shrink(DIALOG_PADDING_PX);
    let heading = Rect::from_min_max(
        inner.min,
        pos2(inner.right(), inner.top() + HEADING_HEIGHT_PX),
    );
    ui.scope_builder(UiBuilder::new().max_rect(heading), |ui| {
        section_heading(ui, MAP_HEADING, None);
    });
    let body_top = heading.bottom() + HEADING_TO_BODY_PX;
    let command_top = inner.bottom() - COMMAND_SIZE_PX.1;
    let list = Rect::from_min_max(
        pos2(inner.left(), body_top),
        pos2(inner.left() + LIST_WIDTH_PX, command_top),
    );
    let current = current_map(profile, launcher.link.rules);
    let mut picked = None;
    ui.scope_builder(UiBuilder::new().max_rect(list), |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        for (index, map) in profile.free_roam_maps.iter().enumerate() {
            let selected = current.is_some_and(|current| current.id == map.id);
            if menu_row(ui, map.label, MenuValue::None, selected, true).clicked() && !selected {
                picked = u8::try_from(index).ok();
            }
        }
    });
    if let Some(index) = picked {
        pick_map(launcher, forms, index);
    }
    let banner_left = list.right() + LIST_TO_BANNER_GAP_PX;
    let banner_width = inner.right() - banner_left;
    let banner_height = (banner_width * BANNER_HEIGHT_PER_WIDTH)
        .min(command_top - BANNER_TO_COMMAND_GAP_PX - body_top);
    let banner = Rect::from_min_max(
        pos2(banner_left, body_top),
        pos2(inner.right(), body_top + banner_height),
    );
    if let Some(map) = current_map(profile, launcher.link.rules) {
        let art = launcher
            .map_banners
            .texture(ui.ctx(), profile.game_id, map.id);
        paint_map_banner(
            ui.painter(),
            banner,
            art.as_ref(),
            map.label,
            DIALOG_NAME_PX,
        );
    }
    let command_row = Rect::from_min_max(pos2(inner.left(), command_top), inner.right_bottom());
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(command_row)
            .layout(Layout::right_to_left(Align::Center)),
        |ui| command_button(ui, DONE_LABEL, COMMAND_SIZE_PX, true, true).clicked(),
    )
    .inner
}

fn pick_map(launcher: &mut Launcher, forms: &NetForms, index: u8) {
    let Some(rules) = rules_with_map(launcher.profile, launcher.link.rules, index) else {
        return;
    };
    forms.net.set_rules(rules);
    launcher.link.rules = rules;
    if let Some(map) = current_map(launcher.profile, rules) {
        launcher
            .log
            .push(format!("lobby: the map is {}", map.label));
    }
}
