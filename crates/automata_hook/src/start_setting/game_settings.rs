use automata_game::character::Character;
use automata_game::partner::{DEFAULT_PARTNER_DRIVE, PartnerDrive};
use automata_game::save_redirect::SaveRole;
use automata_game::start::{StartConfig, StartMode};
use nc_hook::net::{LINK_ENV, game_folder, read_config_text};

use crate::character_setting::{CHARACTER_ENV, ROLE_ENV, choose_character, choose_role};
use crate::start_setting::{
    PARTNER_CHARACTER_ENV, PARTNER_DRIVE_ENV, START_ENV, START_INDEX_ENV, START_SLOT_ENV,
};

#[derive(Debug, Clone, Default)]
pub struct SettingsEnv {
    pub character: Option<String>,
    pub role: Option<String>,
    pub partner_character: Option<String>,
    pub start: Option<String>,
    pub start_index: Option<String>,
    pub start_slot: Option<String>,
    pub link: Option<String>,
    pub partner_drive: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSettings {
    pub role: SaveRole,
    pub character: Character,
    pub partner: Character,
    pub mode: StartMode,
    pub index: usize,
    pub slot: u32,
    pub link_port: Option<u16>,
    pub partner_drive: PartnerDrive,
    pub log_line: String,
}

impl GameSettings {
    pub fn start_config(&self) -> StartConfig {
        StartConfig {
            mode: self.mode,
            index: self.index,
            slot: self.slot,
            hero: self.character,
            partner: self.partner,
        }
    }
}

pub fn choose_game_settings(env: SettingsEnv, config_text: Option<&str>) -> GameSettings {
    let table: toml::Table = config_text
        .and_then(|text| text.parse().ok())
        .unwrap_or_default();
    let role = choose_role(env.role.clone(), config_text);
    let character = choose_character(env.character, env.role, config_text).character;
    let mut notes = Vec::new();
    let partner = choose_partner(
        env.partner_character
            .or_else(|| text_key(&table, "partner_character")),
        role,
        character,
        &mut notes,
    );
    let mode = choose_mode(env.start.or_else(|| text_key(&table, "start")), &mut notes);
    let index = number_setting(
        "start_index",
        env.start_index.or_else(|| text_key(&table, "start_index")),
        &mut notes,
    );
    let slot = number_setting(
        "start_slot",
        env.start_slot.or_else(|| text_key(&table, "start_slot")),
        &mut notes,
    );
    let link_port = choose_link_port(env.link.or_else(|| text_key(&table, "link")), &mut notes);
    let partner_drive = choose_drive(
        env.partner_drive
            .or_else(|| text_key(&table, "partner_drive")),
        &mut notes,
    );
    let mut log_line = format!(
        "settings: role {}, character {}, partner {}, start {}, start_index {index}, start_slot {slot}, partner_drive {}, link {}",
        role.folder_name(),
        character.launcher_id(),
        partner.launcher_id(),
        mode.label(),
        partner_drive.label(),
        link_port.map_or("none".to_owned(), |port| format!("port {port}"))
    );
    if !notes.is_empty() {
        log_line.push_str(&format!(" ({})", notes.join("; ")));
    }
    GameSettings {
        role,
        character,
        partner,
        mode,
        index: index as usize,
        slot,
        link_port,
        partner_drive,
        log_line,
    }
}

pub fn settings_from_launcher() -> GameSettings {
    let config_text = game_folder()
        .and_then(|folder| read_config_text(&folder).ok())
        .flatten();
    let env = SettingsEnv {
        character: std::env::var(CHARACTER_ENV).ok(),
        role: std::env::var(ROLE_ENV).ok(),
        partner_character: std::env::var(PARTNER_CHARACTER_ENV).ok(),
        start: std::env::var(START_ENV).ok(),
        start_index: std::env::var(START_INDEX_ENV).ok(),
        start_slot: std::env::var(START_SLOT_ENV).ok(),
        link: std::env::var(LINK_ENV).ok(),
        partner_drive: std::env::var(PARTNER_DRIVE_ENV).ok(),
    };
    choose_game_settings(env, config_text.as_deref())
}

fn text_key(table: &toml::Table, key: &str) -> Option<String> {
    match table.get(key)? {
        toml::Value::String(text) => Some(text.clone()),
        toml::Value::Integer(number) => Some(number.to_string()),
        _ => None,
    }
}

fn choose_partner(
    given: Option<String>,
    role: SaveRole,
    character: Character,
    notes: &mut Vec<String>,
) -> Character {
    let default = if role == SaveRole::Guest {
        Character::TwoB
    } else {
        Character::NineS
    };
    let wanted = match given.as_deref() {
        Some(text) => Character::from_launcher_id(text).unwrap_or_else(|| {
            notes.push(format!(
                "unknown partner_character {text:?}, using the default"
            ));
            default
        }),
        None => default,
    };
    if wanted == character {
        notes.push("partner was the same as the character, took the first other one".to_owned());
        return Character::ALL
            .into_iter()
            .find(|candidate| *candidate != character)
            .unwrap_or(wanted);
    }
    wanted
}

fn choose_mode(given: Option<String>, notes: &mut Vec<String>) -> StartMode {
    let Some(text) = given else {
        return StartMode::World;
    };
    StartMode::from_text(&text).unwrap_or_else(|| {
        notes.push(format!("unknown start {text:?}, using world"));
        StartMode::World
    })
}

fn choose_link_port(given: Option<String>, notes: &mut Vec<String>) -> Option<u16> {
    let text = given?;
    match text.trim().parse::<u16>() {
        Ok(port) if port != 0 => Some(port),
        _ => {
            notes.push(format!("unreadable link {text:?}, co-op is off"));
            None
        }
    }
}

fn choose_drive(given: Option<String>, notes: &mut Vec<String>) -> PartnerDrive {
    let Some(text) = given else {
        return DEFAULT_PARTNER_DRIVE;
    };
    PartnerDrive::from_text(&text).unwrap_or_else(|| {
        notes.push(format!(
            "unknown partner_drive {text:?}, using {}",
            DEFAULT_PARTNER_DRIVE.label()
        ));
        DEFAULT_PARTNER_DRIVE
    })
}

fn number_setting(key: &str, given: Option<String>, notes: &mut Vec<String>) -> u32 {
    let Some(text) = given else {
        return 0;
    };
    text.trim().parse::<u32>().unwrap_or_else(|_| {
        notes.push(format!("unreadable {key} {text:?}, using 0"));
        0
    })
}
