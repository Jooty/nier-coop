use std::io;
use std::path::Path;

use toml::{Table, Value};

use crate::game_link::{LINK_KEY, NAME_KEY};

pub const GAME_LINK_FILE_NAME: &str = "replicant-coop.toml";

pub fn write_link(game_dir: &Path, port: u16, name: &str) -> io::Result<()> {
    let path = game_dir.join(GAME_LINK_FILE_NAME);
    let mut table = read_table(&path);
    table.insert(LINK_KEY.to_owned(), Value::Integer(i64::from(port)));
    table.insert(NAME_KEY.to_owned(), Value::String(name.to_owned()));
    std::fs::write(path, render(&table))
}

pub fn clear_link(game_dir: &Path) -> io::Result<()> {
    let path = game_dir.join(GAME_LINK_FILE_NAME);
    if !path.exists() {
        return Ok(());
    }
    let mut table = read_table(&path);
    table.remove(LINK_KEY);
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
