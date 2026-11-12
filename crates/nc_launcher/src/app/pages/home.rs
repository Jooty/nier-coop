use eframe::egui::{self, FontId, Rect, Ui, UiBuilder, pos2, vec2};

use crate::app::pages::banner::draw_banner;
use crate::app::pages::lobby::session_open;
use crate::app::palette::{MUTED, TEXT};
use crate::app::style::{body_font, heading_font_family, small_font};
use crate::app::{Launcher, NetForms, WINDOW_TITLE};

const HOME_HEADER_HEIGHT_PX: f32 = 70.0;
const HOME_HEADER_BOTTOM_GAP_PX: f32 = 16.0;
const HOME_TITLE_SIZE_PX: f32 = 38.0;
const HOME_TITLE_TO_SUBTITLE_GAP_PX: f32 = 16.0;
const HOME_SUBTITLE: &str = "Pick a game to host or join a co-op session";
const SUBTITLE_DROP_PX: f32 = 4.0;
const VERSION_LIFT_PX: f32 = 4.0;
const BANNER_GAP_PX: f32 = 16.0;
const BANNERS_BOTTOM_GAP_PX: f32 = 6.0;
const MIN_BANNER_HEIGHT_PX: f32 = 272.0;

pub(super) fn draw_home(ui: &mut Ui, launcher: &mut Launcher, forms: &NetForms) {
    let switch_locked = !launcher.can_switch_game() || session_open(forms);
    ui.spacing_mut().item_spacing.y = 0.0;
    home_header(ui);
    let cards = launcher.cards.clone();
    let count = cards.len().max(1) as f32;
    let room = ui.available_height() - BANNERS_BOTTOM_GAP_PX - BANNER_GAP_PX * (count - 1.0);
    let banner_height = (room / count).max(MIN_BANNER_HEIGHT_PX);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (index, card) in cards.iter().enumerate() {
                if index > 0 {
                    ui.add_space(BANNER_GAP_PX);
                }
                let rect =
                    Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), banner_height));
                ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                    ui.set_min_size(rect.size());
                    draw_banner(ui, rect, launcher, card, switch_locked);
                });
            }
        });
}

fn home_header(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), HOME_HEADER_HEIGHT_PX),
        egui::Sense::hover(),
    );
    let row = Rect::from_min_max(
        pos2(rect.left(), rect.top()),
        pos2(rect.right(), rect.bottom() - HOME_HEADER_BOTTOM_GAP_PX),
    );
    let version = ui.painter().layout_no_wrap(
        format!("launcher {}", env!("CARGO_PKG_VERSION")),
        small_font(),
        MUTED,
    );
    ui.painter().galley(
        pos2(
            row.right() - version.size().x,
            row.bottom() - version.size().y - VERSION_LIFT_PX,
        ),
        version,
        MUTED,
    );
    let title = ui.painter().layout_no_wrap(
        WINDOW_TITLE.to_owned(),
        FontId::new(HOME_TITLE_SIZE_PX, heading_font_family()),
        TEXT,
    );
    let title_rect = Rect::from_min_size(
        pos2(row.left(), row.bottom() - title.size().y),
        title.size(),
    );
    ui.painter().galley(title_rect.min, title, TEXT);
    let subtitle = ui
        .painter()
        .layout_no_wrap(HOME_SUBTITLE.to_owned(), body_font(), MUTED);
    let subtitle_top = title_rect.center().y + SUBTITLE_DROP_PX - subtitle.size().y / 2.0;
    ui.painter().galley(
        pos2(
            title_rect.right() + HOME_TITLE_TO_SUBTITLE_GAP_PX,
            subtitle_top,
        ),
        subtitle,
        MUTED,
    );
}
