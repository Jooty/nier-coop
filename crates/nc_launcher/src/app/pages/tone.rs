use eframe::egui::Color32;

use crate::app::style::{BAD, GOOD, MUTED, WARN};
use crate::home::CardTone;

pub(super) fn tone_color(tone: CardTone) -> Color32 {
    match tone {
        CardTone::Good => GOOD,
        CardTone::Warn => WARN,
        CardTone::Bad => BAD,
        CardTone::Muted => MUTED,
    }
}
