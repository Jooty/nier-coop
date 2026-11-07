use std::io;
use std::path::Path;

use toml::{Table, Value};

use rc_proto::PartnerBody;

use rc_launcher_core::{
    DEFAULT_ENEMY_DAMAGE_PERCENT, DEFAULT_ENEMY_HEALTH_PERCENT, PlayerSettings, SessionOptions,
};

use crate::game_link::{
    BLOCK_SAVES_KEY, BODY_KEY, BOOT_SLOT_KEY, DEV_NO_FOCUS_MENU_KEY, ENEMY_DAMAGE_PERCENT_KEY,
    ENEMY_HEALTH_PERCENT_KEY, EVENT_LOG_KEY, KEEP_RUNNING_KEY, LINK_KEY, LOG_KEY, NAME_KEY,
    NEW_GAME_NAME_KEY, NEW_GAME_SLOT_KEY, SHOW_DISTANCE_KEY, SHOW_NAMES_KEY, SHOW_PARTY_FRAME_KEY,
    SHOW_PING_KEY,
};

pub const GAME_LINK_FILE_NAME: &str = "replicant-coop.toml";

pub fn write_link(game_dir: &Path, port: u16, name: &str) -> io::Result<()> {
    let path = game_dir.join(GAME_LINK_FILE_NAME);
    let mut table = read_table(&path);
    table.insert(LINK_KEY.to_owned(), Value::Integer(i64::from(port)));
    table.insert(NAME_KEY.to_owned(), Value::String(name.to_owned()));
    std::fs::write(path, render(&table))
}

pub fn body_text(body: PartnerBody) -> &'static str {
    match body {
        PartnerBody::Nier => "nier",
        PartnerBody::PrologueCharacter => "prologue",
        PartnerBody::SeedNierY => "nierY",
        PartnerBody::SeedNierF => "nierF",
        PartnerBody::SeedKaineE => "kaine",
    }
}

pub fn write_boot(
    game_dir: &Path,
    slot: Option<u8>,
    block_saves: bool,
    body: Option<PartnerBody>,
) -> io::Result<()> {
    let path = game_dir.join(GAME_LINK_FILE_NAME);
    let mut table = read_table(&path);
    if let Some(body) = body {
        table.insert(
            BODY_KEY.to_owned(),
            Value::String(body_text(body).to_owned()),
        );
    }
    match slot {
        Some(slot) => {
            table.remove(NEW_GAME_SLOT_KEY);
            table.remove(NEW_GAME_NAME_KEY);
            table.insert(BOOT_SLOT_KEY.to_owned(), Value::Integer(i64::from(slot)))
        }
        None => {
            table.remove(NEW_GAME_SLOT_KEY);
            table.remove(NEW_GAME_NAME_KEY);
            table.remove(BOOT_SLOT_KEY)
        }
    };
    if block_saves {
        table.insert(BLOCK_SAVES_KEY.to_owned(), Value::Boolean(true));
    } else {
        table.remove(BLOCK_SAVES_KEY);
    }
    std::fs::write(path, render(&table))
}

pub fn write_new_game(
    game_dir: &Path,
    slot: u8,
    name: &str,
    block_saves: bool,
    body: Option<PartnerBody>,
) -> io::Result<()> {
    let path = game_dir.join(GAME_LINK_FILE_NAME);
    let mut table = read_table(&path);
    if let Some(body) = body {
        table.insert(
            BODY_KEY.to_owned(),
            Value::String(body_text(body).to_owned()),
        );
    }
    table.remove(BOOT_SLOT_KEY);
    table.insert(
        NEW_GAME_SLOT_KEY.to_owned(),
        Value::Integer(i64::from(slot)),
    );
    table.insert(NEW_GAME_NAME_KEY.to_owned(), Value::String(name.to_owned()));
    set_flag(&mut table, BLOCK_SAVES_KEY, block_saves, false);
    std::fs::write(path, render(&table))
}

pub fn write_settings(
    game_dir: &Path,
    settings: &PlayerSettings,
    options: &SessionOptions,
) -> io::Result<()> {
    let path = game_dir.join(GAME_LINK_FILE_NAME);
    let mut table = read_table(&path);
    set_flag(&mut table, SHOW_DISTANCE_KEY, options.show_distance, false);
    set_flag(&mut table, SHOW_PING_KEY, options.show_ping, false);
    set_flag(&mut table, KEEP_RUNNING_KEY, settings.keep_running, false);
    set_flag(&mut table, EVENT_LOG_KEY, settings.event_log, true);
    set_flag(&mut table, SHOW_NAMES_KEY, options.show_names, true);
    set_flag(
        &mut table,
        SHOW_PARTY_FRAME_KEY,
        options.show_party_frame,
        true,
    );
    set_percent(
        &mut table,
        ENEMY_HEALTH_PERCENT_KEY,
        options.enemy_health_percent,
        DEFAULT_ENEMY_HEALTH_PERCENT,
    );
    set_percent(
        &mut table,
        ENEMY_DAMAGE_PERCENT_KEY,
        options.enemy_damage_percent,
        DEFAULT_ENEMY_DAMAGE_PERCENT,
    );
    std::fs::write(path, render(&table))
}

pub fn write_dev_switches(game_dir: &Path, dev_build: bool) -> io::Result<()> {
    let path = game_dir.join(GAME_LINK_FILE_NAME);
    let mut table = read_table(&path);
    set_flag(&mut table, DEV_NO_FOCUS_MENU_KEY, dev_build, false);
    std::fs::write(path, render(&table))
}

pub fn write_log(game_dir: &Path, log_path: &str) -> io::Result<()> {
    let path = game_dir.join(GAME_LINK_FILE_NAME);
    let mut table = read_table(&path);
    table.insert(LOG_KEY.to_owned(), Value::String(log_path.to_owned()));
    std::fs::write(path, render(&table))
}

pub fn clear_boot(game_dir: &Path) -> io::Result<()> {
    remove_keys(
        game_dir,
        &[
            BOOT_SLOT_KEY,
            NEW_GAME_SLOT_KEY,
            NEW_GAME_NAME_KEY,
            BLOCK_SAVES_KEY,
            LOG_KEY,
            KEEP_RUNNING_KEY,
            DEV_NO_FOCUS_MENU_KEY,
        ],
    )
}

pub fn clear_link(game_dir: &Path) -> io::Result<()> {
    remove_keys(game_dir, &[LINK_KEY])
}

fn remove_keys(game_dir: &Path, keys: &[&str]) -> io::Result<()> {
    let path = game_dir.join(GAME_LINK_FILE_NAME);
    if !path.exists() {
        return Ok(());
    }
    let mut table = read_table(&path);
    for key in keys {
        table.remove(*key);
    }
    if table.is_empty() {
        return std::fs::remove_file(path);
    }
    std::fs::write(path, render(&table))
}

fn set_flag(table: &mut Table, key: &str, value: bool, default: bool) {
    if value == default {
        table.remove(key);
    } else {
        table.insert(key.to_owned(), Value::Boolean(value));
    }
}

fn set_percent(table: &mut Table, key: &str, value: u16, default: u16) {
    if value == default {
        table.remove(key);
    } else {
        table.insert(key.to_owned(), Value::Integer(i64::from(value)));
    }
}

fn read_table(path: &Path) -> Table {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}

fn render(table: &Table) -> String {
    toml::to_string(table).unwrap_or_default()
}
