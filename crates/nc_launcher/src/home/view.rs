use nc_launcher_core::banner_facts;

use crate::home::GameCard;
use crate::install::ModStatus;

const INSTALL_MOD_LABEL: &str = "Install the mod";
const UPDATE_MOD_LABEL: &str = "Update the mod";

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
    pub title_note: Option<&'static str>,
    pub tags: &'static [&'static str],
    pub found_text: String,
    pub found_tone: CardTone,
    pub mod_text: String,
    pub mod_tone: CardTone,
    pub saves_text: Option<String>,
    pub greyed_out: bool,
    pub play_enabled: bool,
    pub offers_install: bool,
    pub mod_action: Option<&'static str>,
}

pub fn card_view(card: &GameCard) -> CardView {
    let found = card.install.as_ref().filter(|game| game.exe_present());
    let (found_text, found_tone) = match (&card.install, found) {
        (_, Some(game)) => (game.game_dir.display().to_string(), CardTone::Good),
        (Some(game), None) => (
            format!("No game executable in {}", game.game_dir.display()),
            CardTone::Bad,
        ),
        (None, None) => ("Not found".to_owned(), CardTone::Bad),
    };
    let (mod_text, mod_tone) = mod_line(card);
    let offers_install = card.profile.support.install && found.is_some();
    let mod_action = match card.mod_status {
        ModStatus::NotInstalled => Some(INSTALL_MOD_LABEL),
        ModStatus::Outdated => Some(UPDATE_MOD_LABEL),
        ModStatus::Current | ModStatus::Foreign => None,
    }
    .filter(|_| offers_install);
    let facts = banner_facts(card.profile.game_id);
    CardView {
        title: card.profile.display_name,
        title_note: facts.title_note,
        tags: facts.tags,
        found_text,
        found_tone,
        mod_text,
        mod_tone,
        saves_text: found.and(card.coop_saves).map(saves_line),
        greyed_out: found.is_none(),
        play_enabled: found.is_some(),
        offers_install,
        mod_action,
    }
}

fn saves_line(count: usize) -> String {
    match count {
        0 => "No co-op saves yet".to_owned(),
        1 => "1 co-op save".to_owned(),
        _ => format!("{count} co-op saves"),
    }
}

fn mod_line(card: &GameCard) -> (String, CardTone) {
    if !card.profile.support.install {
        return ("Not built yet".to_owned(), CardTone::Muted);
    }
    let build = card
        .installed_build
        .as_deref()
        .map(|hash| format!(", build {hash}"))
        .unwrap_or_default();
    match card.mod_status {
        ModStatus::NotInstalled => ("Not installed".to_owned(), CardTone::Warn),
        ModStatus::Current => (format!("Installed{build}, up to date"), CardTone::Good),
        ModStatus::Outdated => (
            format!("Installed{build}, a newer build is ready"),
            CardTone::Warn,
        ),
        ModStatus::Foreign => (
            format!(
                "A different {} is in the game folder",
                card.profile.loader_dll_name
            ),
            CardTone::Bad,
        ),
    }
}
