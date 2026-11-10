use crate::home::GameCard;
use crate::install::ModStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardTone {
    Good,
    Warn,
    Bad,
    Muted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardView {
    pub title: &'static str,
    pub found_text: String,
    pub found_tone: CardTone,
    pub mod_text: String,
    pub mod_tone: CardTone,
    pub greyed_out: bool,
    pub play_enabled: bool,
    pub offers_install: bool,
}

pub fn card_view(card: &GameCard) -> CardView {
    let found = card.install.as_ref().filter(|game| game.exe_present());
    let (found_text, found_tone) = match (&card.install, found) {
        (_, Some(game)) => (
            format!("found: {}", game.game_dir.display()),
            CardTone::Good,
        ),
        (Some(game), None) => (
            format!(
                "not found: no game executable in {}",
                game.game_dir.display()
            ),
            CardTone::Bad,
        ),
        (None, None) => ("not found".to_owned(), CardTone::Bad),
    };
    let (mod_text, mod_tone) = mod_line(card);
    CardView {
        title: card.profile.display_name,
        found_text,
        found_tone,
        mod_text,
        mod_tone,
        greyed_out: found.is_none(),
        play_enabled: found.is_some(),
        offers_install: card.profile.support.install && found.is_some(),
    }
}

fn mod_line(card: &GameCard) -> (String, CardTone) {
    if !card.profile.support.install {
        return ("mod not built yet".to_owned(), CardTone::Muted);
    }
    let build = card
        .installed_build
        .as_deref()
        .map(|hash| format!(" ({hash})"))
        .unwrap_or_default();
    match card.mod_status {
        ModStatus::NotInstalled => ("mod not installed".to_owned(), CardTone::Warn),
        ModStatus::Current => (
            format!("mod installed, current build{build}"),
            CardTone::Good,
        ),
        ModStatus::Outdated => (format!("mod installed, older build{build}"), CardTone::Warn),
        ModStatus::Foreign => (
            format!(
                "a different {} is in the game folder",
                card.profile.loader_dll_name
            ),
            CardTone::Bad,
        ),
    }
}
