use automata_game::character::Character;
use automata_game::save_redirect::SaveRole;
use nt_hook::hook_env::read_env;
use nt_hook::net::{game_folder, read_config_text};
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
    let is_guest = role_of(role_text.as_deref()) == SaveRole::Guest;
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

pub fn choose_role(env_role: Option<String>, config_text: Option<&str>) -> SaveRole {
    let keys: CharacterKeys = config_text
        .and_then(|text| toml::from_str(text).ok())
        .unwrap_or_default();
    role_of(env_role.or(keys.role).as_deref())
}

pub fn role_from_launcher() -> SaveRole {
    let config_text = game_folder()
        .and_then(|folder| read_config_text(&folder).ok())
        .flatten();
    choose_role(read_env(ROLE_ENV), config_text.as_deref())
}

fn role_of(role_text: Option<&str>) -> SaveRole {
    if role_text == Some(GUEST_ROLE_TEXT) {
        SaveRole::Guest
    } else {
        SaveRole::Host
    }
}

pub fn character_from_launcher() -> CharacterChoice {
    let config_text = game_folder()
        .and_then(|folder| read_config_text(&folder).ok())
        .flatten();
    choose_character(
        read_env(CHARACTER_ENV),
        read_env(ROLE_ENV),
        config_text.as_deref(),
    )
}
