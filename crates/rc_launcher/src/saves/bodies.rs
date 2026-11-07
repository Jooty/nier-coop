use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use rc_game::save_files::COOP_GAMEDATA_FILE_NAME;
use rc_proto::PartnerBody;
use serde::{Deserialize, Serialize};

use crate::saves::coop_gamedata_path;

const COOP_BODIES_SUFFIX: &str = ".bodies.toml";

pub type SlotBodies = BTreeMap<u8, PartnerBody>;

#[derive(Default, Serialize, Deserialize)]
struct BodiesFile {
    #[serde(default)]
    slots: BTreeMap<String, PartnerBody>,
}

pub fn coop_bodies_path(save_dir: &Path) -> Option<PathBuf> {
    let coop = coop_gamedata_path(save_dir)?;
    Some(coop.with_file_name(format!("{COOP_GAMEDATA_FILE_NAME}{COOP_BODIES_SUFFIX}")))
}

pub fn parse_slot_bodies(text: &str) -> SlotBodies {
    let file: BodiesFile = toml::from_str(text).unwrap_or_default();
    file.slots
        .into_iter()
        .filter_map(|(slot, body)| Some((slot.parse().ok()?, body)))
        .collect()
}

pub fn render_slot_bodies(bodies: &SlotBodies) -> String {
    let file = BodiesFile {
        slots: bodies
            .iter()
            .map(|(slot, body)| (slot.to_string(), *body))
            .collect(),
    };
    toml::to_string(&file).unwrap_or_default()
}

pub fn read_slot_bodies(path: &Path) -> SlotBodies {
    std::fs::read_to_string(path)
        .map(|text| parse_slot_bodies(&text))
        .unwrap_or_default()
}

pub fn slot_body(bodies: &SlotBodies, slot: u8) -> PartnerBody {
    bodies.get(&slot).copied().unwrap_or_default()
}

pub fn store_slot_body(path: &Path, slot: u8, body: PartnerBody) -> io::Result<()> {
    let mut bodies = read_slot_bodies(path);
    bodies.insert(slot, body);
    let staging = path.with_extension("tmp");
    std::fs::write(&staging, render_slot_bodies(&bodies))?;
    std::fs::rename(&staging, path)
}
