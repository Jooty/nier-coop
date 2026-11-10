use automata_game::character::Character;
use nc_hook::net::{game_folder, read_config_text};
use serde::Deserialize;

use crate::character_setting::{CHARACTER_ENV, GUEST_ROLE_TEXT, HOST_ROLE_TEXT, ROLE_ENV};

#[derive(Deserialize, Default)]
struct CharacterKeys {
    character: Option<String>,
    role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterChoice {
    pub character: Character,
    pub log_line: String,
}

pub fn choose_character(
    env_character: Option<String>,
    env_role: Option<String>,
    config_text: Option<&str>,
) -> CharacterChoice {
    let keys: CharacterKeys = config_text
        .and_then(|text| toml::from_str(text).ok())
        .unwrap_or_default();
    let role_text = env_role.or(keys.role);
    let is_guest = role_text.as_deref() == Some(GUEST_ROLE_TEXT);
    let role_name = if is_guest {
        GUEST_ROLE_TEXT
    } else {
        HOST_ROLE_TEXT
    };
    let fallback = if is_guest {
        Character::NineS
    } else {
        Character::TwoB
    };
    let given = env_character.or(keys.character);
    let (character, log_line) = match given.as_deref() {
        Some(text) => match Character::from_launcher_id(text) {
            Some(character) => (
                character,
                format!(
                    "character: {} from the launcher ({role_name})",
                    character.launcher_id()
                ),
            ),
            None => (
                fallback,
                format!(
                    "character: unknown value {text:?} from the launcher, using {} ({role_name})",
                    fallback.launcher_id()
                ),
            ),
        },
        None => (
            fallback,
            format!(
                "character: the launcher gave none, using {} ({role_name})",
                fallback.launcher_id()
            ),
        ),
    };
    CharacterChoice {
        character,
        log_line,
    }
}

pub fn character_from_launcher() -> CharacterChoice {
    let config_text = game_folder()
        .and_then(|folder| read_config_text(&folder).ok())
        .flatten();
    choose_character(
        std::env::var(CHARACTER_ENV).ok(),
        std::env::var(ROLE_ENV).ok(),
        config_text.as_deref(),
    )
}
