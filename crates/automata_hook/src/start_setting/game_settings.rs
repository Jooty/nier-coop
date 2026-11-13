use automata_game::autopilot::Autopilot;
use automata_game::character::Character;
use automata_game::downed::downed_revive_from_text;
use automata_game::enemies::EnemyShare;
use automata_game::partner::{DEFAULT_PARTNER_DRIVE, PartnerDrive};
use automata_game::save_redirect::SaveRole;
use automata_game::start::{HIGHEST_SAVE_SLOT, StartConfig, StartMode, route_lead};
use automata_game::story_gate::StoryRule;
use nc_hook::net::{LINK_ENV, game_folder, read_config_text};

use crate::character_setting::{CHARACTER_ENV, ROLE_ENV, choose_character, choose_role};
use crate::start_setting::PARTNER_FLIGHT_UNIT_ENV;
use crate::start_setting::{
    AUTOPILOT_ENV, DOWNED_REVIVE_ENV, EVENTS_STARTED_FOR_THE_PARTNER_ENV, EVENTS_TOGETHER_ENV,
    EVERY_LOAD_TOGETHER_ENV, ONE_LOADING_SCREEN_ENV, PARTNER_CHARACTER_ENV, PARTNER_DRIVE_ENV,
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
    pub every_load_together: Option<String>,
    pub shared_enemies: Option<String>,
    pub downed_revive: Option<String>,
    pub events_together: Option<String>,
    pub events_started_for_the_partner: Option<String>,
    pub partner_flight_unit: Option<String>,
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
    pub every_load_together: bool,
    pub enemy_share: EnemyShare,
    pub downed_revive: bool,
    pub log_line: String,
    pub events_together: bool,
    pub events_started_for_the_partner: bool,
    pub partner_flight_unit: bool,
}

impl GameSettings {
    pub fn start_config(&self) -> StartConfig {
        StartConfig {
            mode: self.mode,
            index: self.index,
            slot: self.slot,
            hero: self.character,
            partner: self.partner,
            route_lead: route_lead(self.role, self.character, self.partner, self.enemy_share),
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
    if matches!(mode, StartMode::Slot | StartMode::NewGame) && slot > HIGHEST_SAVE_SLOT {
        notes.push(format!(
            "start_slot {slot} is not a save slot (0 to {HIGHEST_SAVE_SLOT}), the start will be refused"
        ));
    }
    let story_given = env.story.or_else(|| text_key(&table, "story"));
    let story_is_given = story_given.is_some();
    let mut story = choose_story(story_given, &mut notes);
    if matches!(mode, StartMode::NewGame | StartMode::Continue) {
        if !story_is_given {
            story = StoryRule::Follow;
        } else if story == StoryRule::Hold {
            notes.push(format!(
                "story hold does not fit the start {}, the story is allowed",
                mode.label()
            ));
            story = StoryRule::Allow;
        }
    }
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
    let every_load_together = on_unless_turned_off(
        "every_load_together",
        env.every_load_together
            .or_else(|| text_key(&table, "every_load_together")),
        &mut notes,
    );
    let enemy_share = choose_enemy_share(
        env.shared_enemies
            .or_else(|| text_key(&table, "shared_enemies")),
        &mut notes,
    );
    let downed_revive = choose_downed_revive(
        env.downed_revive
            .or_else(|| text_key(&table, "downed_revive")),
        &mut notes,
    );
    let events_together = on_unless_turned_off(
        "events_together",
        env.events_together
            .or_else(|| text_key(&table, "events_together")),
        &mut notes,
    );
    let events_started_for_the_partner = on_unless_turned_off(
        "events_started_for_the_partner",
        env.events_started_for_the_partner
            .or_else(|| text_key(&table, "events_started_for_the_partner")),
        &mut notes,
    );
    let partner_flight_unit = on_unless_turned_off(
        "partner_flight_unit",
        env.partner_flight_unit
            .or_else(|| text_key(&table, "partner_flight_unit")),
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
    if !every_load_together {
        log_line.push_str(", every_load_together off");
    }
    log_line.push_str(&format!(", shared_enemies {}", enemy_share.label()));
    if !downed_revive {
        log_line.push_str(", downed_revive off");
    }
    if !events_together {
        log_line.push_str(", events_together off");
    }
    if !events_started_for_the_partner {
        log_line.push_str(", events_started_for_the_partner off");
    }
    if !partner_flight_unit {
        log_line.push_str(", partner_flight_unit off");
    }
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
        every_load_together,
        enemy_share,
        downed_revive,
        log_line,
        events_together,
        events_started_for_the_partner,
        partner_flight_unit,
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
        every_load_together: std::env::var(EVERY_LOAD_TOGETHER_ENV).ok(),
        shared_enemies: std::env::var(SHARED_ENEMIES_ENV).ok(),
        downed_revive: std::env::var(DOWNED_REVIVE_ENV).ok(),
        events_together: std::env::var(EVENTS_TOGETHER_ENV).ok(),
        events_started_for_the_partner: std::env::var(EVENTS_STARTED_FOR_THE_PARTNER_ENV).ok(),
        partner_flight_unit: std::env::var(PARTNER_FLIGHT_UNIT_ENV).ok(),
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
        notes.push(format!("unknown shared_enemies {text:?}, using on"));
        EnemyShare::default()
    })
}

fn choose_downed_revive(given: Option<String>, notes: &mut Vec<String>) -> bool {
    let Some(text) = given else {
        return true;
    };
    downed_revive_from_text(&text).unwrap_or_else(|| {
        notes.push(format!("unknown downed_revive {text:?}, using on"));
        true
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
