use eframe::egui::{ScrollArea, Ui, UiBuilder};
use nc_launcher_core::current_mode;

use crate::app::pages::lobby::{draw_lobby, session_open};
use crate::app::pages::lobby_layout::lobby_layout;
use crate::app::pages::lobby_options::draw_lobby_options;
use crate::app::pages::member_list::draw_member_list;
use crate::app::pages::mode_menu::draw_mode_menu;
use crate::app::pages::player_seats::{SeatFacts, draw_player_seats};
use crate::app::{Launcher, NetForms};
use crate::net::NetState;

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
    let campaign =
        current_mode(launcher.profile, launcher.link.rules).is_none_or(|mode| mode.is_campaign());
    let state = forms.net.status().state;
    let peer_name = match &state {
        NetState::Connected { peer_name } => Some(peer_name.as_str()),
        _ => None,
    };
    let facts = SeatFacts {
        peer_name,
        name_editable: !session_open(forms),
    };
    ui.scope_builder(UiBuilder::new().max_rect(layout.main), |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if campaign {
            draw_player_seats(ui, launcher, facts);
        } else {
            draw_member_list(ui, launcher);
        }
    });
    ui.scope_builder(UiBuilder::new().max_rect(layout.bar), |ui| {
        draw_lobby(ui, launcher, forms);
    });
}
