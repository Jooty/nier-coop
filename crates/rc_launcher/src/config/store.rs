use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use rc_proto::PartnerBody;

use rc_launcher_core::{
    DEFAULT_ENEMY_DAMAGE_PERCENT, DEFAULT_ENEMY_HEALTH_PERCENT, MAX_ENEMY_DAMAGE_PERCENT,
    MAX_ENEMY_HEALTH_PERCENT, PlayerSettings, SessionOptions,
};

use crate::config::{APP_DIR_NAME, CONFIG_FILE_NAME};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LauncherConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_name: Option<String>,
    #[serde(default, skip_serializing_if = "is_nier")]
    pub body: PartnerBody,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub show_distance: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub show_ping: bool,
    #[serde(default = "enabled", skip_serializing_if = "is_enabled")]
    pub keep_running: bool,
    #[serde(default = "enabled", skip_serializing_if = "is_enabled")]
    pub event_log: bool,
    #[serde(default = "enabled", skip_serializing_if = "is_enabled")]
    pub show_names: bool,
    #[serde(default = "enabled", skip_serializing_if = "is_enabled")]
    pub show_party_frame: bool,
    #[serde(
        default = "default_enemy_health_percent",
        skip_serializing_if = "is_default_enemy_health_percent"
    )]
    pub enemy_health_percent: u16,
    #[serde(
        default = "default_enemy_damage_percent",
        skip_serializing_if = "is_default_enemy_damage_percent"
    )]
    pub enemy_damage_percent: u16,
    #[serde(default = "enabled", skip_serializing_if = "is_enabled")]
    pub check_for_updates: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_url: Option<String>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            game_dir: None,
            player_name: None,
            body: PartnerBody::Nier,
            show_distance: false,
            show_ping: false,
            keep_running: true,
            event_log: true,
            show_names: true,
            show_party_frame: true,
            enemy_health_percent: DEFAULT_ENEMY_HEALTH_PERCENT,
            enemy_damage_percent: DEFAULT_ENEMY_DAMAGE_PERCENT,
            check_for_updates: true,
            update_url: None,
        }
    }
}

impl LauncherConfig {
    pub fn parse(text: &str) -> Self {
        toml::from_str(text).unwrap_or_default()
    }

    pub fn render(&self) -> String {
        toml::to_string(self).unwrap_or_default()
    }

    pub fn player_settings(&self, force_keep_running: bool) -> PlayerSettings {
        PlayerSettings {
            keep_running: self.keep_running || force_keep_running,
            event_log: self.event_log,
        }
    }

    pub fn session_options(&self) -> SessionOptions {
        SessionOptions {
            show_names: self.show_names,
            show_party_frame: self.show_party_frame,
            show_distance: self.show_distance,
            show_ping: self.show_ping,
            enemy_health_percent: self.enemy_health_percent,
            enemy_damage_percent: self.enemy_damage_percent,
        }
        .clamped()
    }

    pub fn take_player_settings(&mut self, settings: PlayerSettings) {
        self.keep_running = settings.keep_running;
        self.event_log = settings.event_log;
    }

    pub fn take_session_options(&mut self, options: SessionOptions) {
        self.show_names = options.show_names;
        self.show_party_frame = options.show_party_frame;
        self.show_distance = options.show_distance;
        self.show_ping = options.show_ping;
        self.enemy_health_percent = options.enemy_health_percent.min(MAX_ENEMY_HEALTH_PERCENT);
        self.enemy_damage_percent = options.enemy_damage_percent.min(MAX_ENEMY_DAMAGE_PERCENT);
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.render())
    }
}

pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(APP_DIR_NAME))
}

pub fn config_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join(CONFIG_FILE_NAME))
}

pub fn cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|dir| dir.join(APP_DIR_NAME))
}

pub fn data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join(APP_DIR_NAME))
}

fn enabled() -> bool {
    true
}

fn is_enabled(value: &bool) -> bool {
    *value
}

fn default_enemy_health_percent() -> u16 {
    DEFAULT_ENEMY_HEALTH_PERCENT
}

fn default_enemy_damage_percent() -> u16 {
    DEFAULT_ENEMY_DAMAGE_PERCENT
}

fn is_default_enemy_health_percent(value: &u16) -> bool {
    *value == DEFAULT_ENEMY_HEALTH_PERCENT
}

fn is_default_enemy_damage_percent(value: &u16) -> bool {
    *value == DEFAULT_ENEMY_DAMAGE_PERCENT
}

fn is_nier(body: &PartnerBody) -> bool {
    *body == PartnerBody::Nier
}
