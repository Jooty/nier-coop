use crate::net::LOBBY_MODE_CONTINUE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchMode {
    NewGame { name: String },
    Continue { slot: u8 },
}

pub fn resolve_mode(
    mode: u8,
    continue_slot: Option<u8>,
    new_game_name: Option<String>,
) -> Option<LaunchMode> {
    if mode == LOBBY_MODE_CONTINUE
        && let Some(slot) = continue_slot
    {
        return Some(LaunchMode::Continue { slot });
    }
    new_game_name.map(|name| LaunchMode::NewGame { name })
}
