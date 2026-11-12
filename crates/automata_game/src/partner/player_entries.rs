use crate::mode::SessionMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerEntry {
    pub slot: u8,
    pub name: String,
}

/// The players the screen and the menu show one entry for, in slot order: the campaign shows
/// only the lowest other slot (its one partner), free roam shows every other player.
pub fn player_entries(mode: SessionMode, others: &[PlayerEntry]) -> Vec<PlayerEntry> {
    let mut entries = others.to_vec();
    entries.sort_by_key(|entry| entry.slot);
    entries.dedup_by_key(|entry| entry.slot);
    if mode.story_rules_apply() {
        entries.truncate(1);
    }
    entries
}
