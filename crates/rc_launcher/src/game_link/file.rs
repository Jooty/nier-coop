use std::io;
use std::path::Path;

use toml::{Table, Value};

use crate::game_link::{BLOCK_SAVES_KEY, BOOT_SLOT_KEY, LINK_KEY, NAME_KEY};

pub const GAME_LINK_FILE_NAME: &str = "replicant-coop.toml";

pub fn write_link(game_dir: &Path, port: u16, name: &str) -> io::Result<()> {
    let path = game_dir.join(GAME_LINK_FILE_NAME);
    let mut table = read_table(&path);
    table.insert(LINK_KEY.to_owned(), Value::Integer(i64::from(port)));
    table.insert(NAME_KEY.to_owned(), Value::String(name.to_owned()));
    std::fs::write(path, render(&table))
}

pub fn write_boot(game_dir: &Path, slot: Option<u8>, block_saves: bool) -> io::Result<()> {
    let path = game_dir.join(GAME_LINK_FILE_NAME);
    let mut table = read_table(&path);
    match slot {
        Some(slot) => table.insert(BOOT_SLOT_KEY.to_owned(), Value::Integer(i64::from(slot))),
        None => table.remove(BOOT_SLOT_KEY),
    };
    if block_saves {
        table.insert(BLOCK_SAVES_KEY.to_owned(), Value::Boolean(true));
    } else {
        table.remove(BLOCK_SAVES_KEY);
    }
    std::fs::write(path, render(&table))
}

pub fn clear_boot(game_dir: &Path) -> io::Result<()> {
    remove_keys(game_dir, &[BOOT_SLOT_KEY, BLOCK_SAVES_KEY])
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

fn read_table(path: &Path) -> Table {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}

fn render(table: &Table) -> String {
    toml::to_string(table).unwrap_or_default()
}
