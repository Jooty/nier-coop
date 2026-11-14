use automata_game::autopilot::Autopilot;
use automata_game::character::Character;
use automata_game::downed::downed_revive_from_text;
use automata_game::enemies::EnemyShare;
use automata_game::partner::{DEFAULT_PARTNER_DRIVE, PartnerDrive};
use automata_game::save_redirect::SaveRole;
use automata_game::start::{HIGHEST_SAVE_SLOT, StartConfig, StartMode, route_lead};
use automata_game::story_gate::StoryRule;
use automata_game::story_steps::StoryStepsMode;
use nt_hook::hook_env::{log_old_env_names_in_use, read_env};
use nt_hook::net::{LINK_ENV, game_folder, read_config_text};

use crate::character_setting::{CHARACTER_ENV, ROLE_ENV, choose_character, choose_role};
use crate::start_setting::ENEMY_COUNTS_FOLLOW_THE_HOST_ENV;
use crate::start_setting::EVENT_PAUSE_TOGETHER_ENV;
use crate::start_setting::PARTNER_BODY_KEEPS_ITS_SLOT_ENV;
use crate::start_setting::PARTNER_FLIGHT_TAKE_OVER_ENV;
use crate::start_setting::SHARED_DEFEAT_ENV;
use crate::start_setting::SKIP_VOTE_ENV;
use crate::start_setting::STORY_STEPS_ENV;
use crate::start_setting::WATCHED_HERO_FOLLOWS_FLIGHT_ENV;
use crate::start_setting::{
    AUTOPILOT_ENV, DOWNED_REVIVE_ENV, EVENTS_STARTED_FOR_THE_PARTNER_ENV, EVENTS_TOGETHER_ENV,
    EVERY_LOAD_TOGETHER_ENV, ONE_LOADING_SCREEN_ENV, PARTNER_CHARACTER_ENV, PARTNER_DRIVE_ENV,
    SHARED_ENEMIES_ENV, START_ENV, START_INDEX_ENV, START_SLOT_ENV, START_TOGETHER_ENV, STORY_ENV,
};
use crate::start_setting::{GUEST_WATCHES_THE_OPENING_ENV, WATCHING_ENDS_AT_ENV};
use crate::start_setting::{PARTNER_FLIGHT_UNIT_ENV, SAVE_TOGETHER_ENV};

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
    pub save_together: Option<String>,
    pub skip_vote: Option<String>,
    pub partner_flight_take_over: Option<String>,
    pub enemy_counts_follow_the_host: Option<String>,
    pub shared_defeat: Option<String>,
    pub story_steps: Option<String>,
    pub guest_watches_the_opening: Option<String>,
    pub watching_ends_at: Option<String>,
    pub partner_body_keeps_its_slot: Option<String>,
    pub watched_hero_follows_flight: Option<String>,
    pub event_pause_together: Option<String>,
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
    pub save_together: bool,
    pub skip_vote: bool,
    pub partner_flight_take_over: bool,
    pub enemy_counts_follow_the_host: bool,
    pub shared_defeat: bool,
    pub story_steps: StoryStepsMode,
    pub guest_watches_the_opening: bool,
    pub watching_ends_at: Option<String>,
    pub partner_body_keeps_its_slot: bool,
    pub story_steps_on_by_campaign_default: bool,
    pub watched_hero_follows_flight: bool,
    pub event_pause_together: bool,
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
        if story == StoryRule::Allow {
            notes.push(format!(
                "the host does not lead the story in this session: the given story rule is allow, not follow, for the start {}",
                mode.label()
            ));
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
    let save_together = on_unless_turned_off(
        "save_together",
        env.save_together
            .or_else(|| text_key(&table, "save_together")),
        &mut notes,
    );
    let skip_vote = on_unless_turned_off(
        "skip_vote",
        env.skip_vote.or_else(|| text_key(&table, "skip_vote")),
        &mut notes,
    );
    let partner_flight_take_over = on_unless_turned_off(
        "partner_flight_take_over",
        env.partner_flight_take_over
            .or_else(|| text_key(&table, "partner_flight_take_over")),
        &mut notes,
    );
    let enemy_counts_follow_the_host = on_unless_turned_off(
        "enemy_counts_follow_the_host",
        env.enemy_counts_follow_the_host
            .or_else(|| text_key(&table, "enemy_counts_follow_the_host")),
        &mut notes,
    );
    let shared_defeat = on_unless_turned_off(
        "shared_defeat",
        env.shared_defeat
            .or_else(|| text_key(&table, "shared_defeat")),
        &mut notes,
    );
    let story_steps_given = env.story_steps.or_else(|| text_key(&table, "story_steps"));
    let is_campaign_start = matches!(mode, StartMode::NewGame | StartMode::Continue);
    let story_steps_on_by_campaign_default = story_steps_given.is_none() && is_campaign_start;
    let story_steps = choose_story_steps(story_steps_given, is_campaign_start, &mut notes);
    let guest_watches_the_opening = on_unless_turned_off(
        "guest_watches_the_opening",
        env.guest_watches_the_opening
            .or_else(|| text_key(&table, "guest_watches_the_opening")),
        &mut notes,
    );
    let watching_ends_at = env
        .watching_ends_at
        .or_else(|| text_key(&table, "watching_ends_at"))
        .map(|end| end.trim().to_owned())
        .filter(|end| !end.is_empty());
    let partner_body_keeps_its_slot = on_unless_turned_off(
        "partner_body_keeps_its_slot",
        env.partner_body_keeps_its_slot
            .or_else(|| text_key(&table, "partner_body_keeps_its_slot")),
        &mut notes,
    );
    let watched_hero_follows_flight = on_unless_turned_off(
        "watched_hero_follows_flight",
        env.watched_hero_follows_flight
            .or_else(|| text_key(&table, "watched_hero_follows_flight")),
        &mut notes,
    );
    let event_pause_together = on_unless_turned_off(
        "event_pause_together",
        env.event_pause_together
            .or_else(|| text_key(&table, "event_pause_together")),
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
    if !save_together {
        log_line.push_str(", save_together off");
    }
    if !skip_vote {
        log_line.push_str(", skip_vote off");
    }
    if !partner_flight_take_over {
        log_line.push_str(", partner_flight_take_over off");
    }
    if !enemy_counts_follow_the_host {
        log_line.push_str(", enemy_counts_follow_the_host off");
    }
    if !shared_defeat {
        log_line.push_str(", shared_defeat off");
    }
    if story_steps != StoryStepsMode::default() || is_campaign_start {
        log_line.push_str(&format!(", story_steps {}", story_steps.label()));
    }
    if !guest_watches_the_opening {
        log_line.push_str(", guest_watches_the_opening off");
    }
    if let Some(end) = &watching_ends_at {
        log_line.push_str(&format!(", watching_ends_at {end}"));
    }
    if !partner_body_keeps_its_slot {
        log_line.push_str(", partner_body_keeps_its_slot off");
    }
    if !watched_hero_follows_flight {
        log_line.push_str(", watched_hero_follows_flight off");
    }
    if !event_pause_together {
        log_line.push_str(", event_pause_together off");
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
        save_together,
        skip_vote,
        partner_flight_take_over,
        enemy_counts_follow_the_host,
        shared_defeat,
        story_steps,
        guest_watches_the_opening,
        watching_ends_at,
        partner_body_keeps_its_slot,
        story_steps_on_by_campaign_default,
        watched_hero_follows_flight,
        event_pause_together,
    }
}

pub fn settings_from_launcher() -> GameSettings {
    let config_text = game_folder()
        .and_then(|folder| read_config_text(&folder).ok())
        .flatten();
    let env = SettingsEnv {
        character: read_env(CHARACTER_ENV),
        role: read_env(ROLE_ENV),
        partner_character: read_env(PARTNER_CHARACTER_ENV),
        start: read_env(START_ENV),
        start_index: read_env(START_INDEX_ENV),
        start_slot: read_env(START_SLOT_ENV),
        link: read_env(LINK_ENV),
        partner_drive: read_env(PARTNER_DRIVE_ENV),
        story: read_env(STORY_ENV),
        autopilot: read_env(AUTOPILOT_ENV),
        start_together: read_env(START_TOGETHER_ENV),
        one_loading_screen: read_env(ONE_LOADING_SCREEN_ENV),
        every_load_together: read_env(EVERY_LOAD_TOGETHER_ENV),
        shared_enemies: read_env(SHARED_ENEMIES_ENV),
        downed_revive: read_env(DOWNED_REVIVE_ENV),
        events_together: read_env(EVENTS_TOGETHER_ENV),
        events_started_for_the_partner: read_env(EVENTS_STARTED_FOR_THE_PARTNER_ENV),
        partner_flight_unit: read_env(PARTNER_FLIGHT_UNIT_ENV),
        save_together: read_env(SAVE_TOGETHER_ENV),
        skip_vote: read_env(SKIP_VOTE_ENV),
        partner_flight_take_over: read_env(PARTNER_FLIGHT_TAKE_OVER_ENV),
        enemy_counts_follow_the_host: read_env(ENEMY_COUNTS_FOLLOW_THE_HOST_ENV),
        shared_defeat: read_env(SHARED_DEFEAT_ENV),
        story_steps: read_env(STORY_STEPS_ENV),
        guest_watches_the_opening: read_env(GUEST_WATCHES_THE_OPENING_ENV),
        watching_ends_at: read_env(WATCHING_ENDS_AT_ENV),
        partner_body_keeps_its_slot: read_env(PARTNER_BODY_KEEPS_ITS_SLOT_ENV),
        watched_hero_follows_flight: read_env(WATCHED_HERO_FOLLOWS_FLIGHT_ENV),
        event_pause_together: read_env(EVENT_PAUSE_TOGETHER_ENV),
    };
    let settings = choose_game_settings(env, config_text.as_deref());
    log_old_env_names_in_use();
    settings
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

fn choose_story_steps(
    given: Option<String>,
    is_campaign_start: bool,
    notes: &mut Vec<String>,
) -> StoryStepsMode {
    let Some(text) = given else {
        return if is_campaign_start {
            StoryStepsMode::On
        } else {
            StoryStepsMode::default()
        };
    };
    StoryStepsMode::from_text(&text).unwrap_or_else(|| {
        notes.push(format!(
            "unknown story_steps {text:?}, using {}",
            StoryStepsMode::default().label()
        ));
        StoryStepsMode::default()
    })
}
