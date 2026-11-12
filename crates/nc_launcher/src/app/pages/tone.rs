use eframe::egui::Color32;

use crate::app::palette::{INK_MUTED, NOT_READY, READY};
use crate::home::CardTone;

pub(super) fn tone_color(tone: CardTone) -> Color32 {
    match tone {
        CardTone::Good => READY,
        CardTone::Warn => NOT_READY,
        CardTone::Bad => NOT_READY,
        CardTone::Muted => INK_MUTED,
    }
}
