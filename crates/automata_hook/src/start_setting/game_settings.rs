use automata_game::autopilot::Autopilot;
use automata_game::character::Character;
use automata_game::enemies::EnemyShare;
use automata_game::partner::{DEFAULT_PARTNER_DRIVE, PartnerDrive};
use automata_game::save_redirect::SaveRole;
use automata_game::start::{HIGHEST_SAVE_SLOT, StartConfig, StartMode};
use automata_game::story_gate::StoryRule;
use nc_hook::net::{LINK_ENV, game_folder, read_config_text};

use crate::character_setting::{CHARACTER_ENV, ROLE_ENV, choose_character, choose_role};
use crate::start_setting::{
    AUTOPILOT_ENV, ONE_LOADING_SCREEN_ENV, PARTNER_CHARACTER_ENV, PARTNER_DRIVE_ENV,
    SHARED_ENEMIES_ENV, START_ENV, START_INDEX_ENV, START_SLOT_ENV, START_TOGETHER_ENV, STORY_ENV,
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
    pub story: Option<String>,
    pub autopilot: Option<String>,
    pub start_together: Option<String>,
    pub one_loading_screen: Option<String>,
    pub shared_enemies: Option<String>,
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
    pub story: StoryRule,
    pub autopilot: Autopilot,
    pub start_together: bool,
    pub one_loading_screen: bool,
    pub enemy_share: EnemyShare,
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
    if mode == StartMode::Slot && slot > HIGHEST_SAVE_SLOT {
        notes.push(format!(
            "start_slot {slot} is not a save slot (0 to {HIGHEST_SAVE_SLOT}), the start will be refused"
        ));
    }
    let story = choose_story(env.story.or_else(|| text_key(&table, "story")), &mut notes);
    let link_port = choose_link_port(env.link.or_else(|| text_key(&table, "link")), &mut notes);
    let partner_drive = choose_drive(
        env.partner_drive
            .or_else(|| text_key(&table, "partner_drive")),
        &mut notes,
    );
    let autopilot = choose_autopilot(
        env.autopilot.or_else(|| text_key(&table, "autopilot")),
        &mut notes,
    );
    let start_together = on_unless_turned_off(
        "start_together",
        env.start_together
            .or_else(|| text_key(&table, "start_together")),
        &mut notes,
    );
    let one_loading_screen = on_unless_turned_off(
        "one_loading_screen",
        env.one_loading_screen
            .or_else(|| text_key(&table, "one_loading_screen")),
        &mut notes,
    );
    let enemy_share = choose_enemy_share(
        env.shared_enemies
            .or_else(|| text_key(&table, "shared_enemies")),
        &mut notes,
    );
    let mut log_line = format!(
        "settings: role {}, character {}, partner {}, start {}, start_index {index}, start_slot {slot}, story {}, partner_drive {}, link {}",
        role.folder_name(),
        character.launcher_id(),
        partner.launcher_id(),
        mode.label(),
        story.label(),
        partner_drive.label(),
        link_port.map_or("none".to_owned(), |port| format!("port {port}"))
    );
    if autopilot != Autopilot::Off {
        log_line.push_str(&format!(", autopilot {}", autopilot.label()));
    }
    if !start_together {
        log_line.push_str(", start_together off");
    }
    if !one_loading_screen {
        log_line.push_str(", one_loading_screen off");
    }
    log_line.push_str(&format!(", shared_enemies {}", enemy_share.label()));
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
        story,
        autopilot,
        start_together,
        one_loading_screen,
        enemy_share,
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
        story: std::env::var(STORY_ENV).ok(),
        autopilot: std::env::var(AUTOPILOT_ENV).ok(),
        start_together: std::env::var(START_TOGETHER_ENV).ok(),
        one_loading_screen: std::env::var(ONE_LOADING_SCREEN_ENV).ok(),
        shared_enemies: std::env::var(SHARED_ENEMIES_ENV).ok(),
    };
    choose_game_settings(env, config_text.as_deref())
}

fn text_key(table: &toml::Table, key: &str) -> Option<String> {
    match table.get(key)? {
        toml::Value::String(text) => Some(text.clone()),
        toml::Value::Integer(number) => Some(number.to_string()),
        toml::Value::Boolean(flag) => Some(if *flag { "on" } else { "off" }.to_owned()),
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

fn choose_story(given: Option<String>, notes: &mut Vec<String>) -> StoryRule {
    let Some(text) = given else {
        return StoryRule::default();
    };
    StoryRule::from_text(&text).unwrap_or_else(|| {
        notes.push(format!("unknown story {text:?}, using hold"));
        StoryRule::default()
    })
}

fn choose_autopilot(given: Option<String>, notes: &mut Vec<String>) -> Autopilot {
    let Some(text) = given else {
        return Autopilot::Off;
    };
    Autopilot::from_text(&text).unwrap_or_else(|| {
        notes.push(format!("unknown autopilot {text:?}, using off"));
        Autopilot::Off
    })
}

fn on_unless_turned_off(key: &str, given: Option<String>, notes: &mut Vec<String>) -> bool {
    match given.as_deref() {
        None | Some("on") => true,
        Some("off") => false,
        Some(text) => {
            notes.push(format!("unknown {key} {text:?}, using on"));
            true
        }
    }
}

fn choose_enemy_share(given: Option<String>, notes: &mut Vec<String>) -> EnemyShare {
    let Some(text) = given else {
        return EnemyShare::default();
    };
    EnemyShare::from_text(&text).unwrap_or_else(|| {
        notes.push(format!("unknown shared_enemies {text:?}, using watch"));
        EnemyShare::default()
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
