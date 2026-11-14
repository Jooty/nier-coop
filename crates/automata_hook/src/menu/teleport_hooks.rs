use automata_game::partner::PlayerEntry;
use automata_game::travel::{TravelPoint, TravelView};

/// What the Teleport section reads and asks; the game side fills it in on Windows.
#[derive(Debug, Clone, Copy)]
pub struct TeleportHooks {
    pub view: fn() -> TravelView,
    pub ask_jump: fn(TravelPoint) -> Result<(), String>,
    pub ask_go_to_partner: fn() -> Result<String, String>,
    pub ask_bring_partner: fn() -> Result<String, String>,
    pub players: fn() -> Vec<PlayerEntry>,
    pub ask_go_to_player: fn(u8) -> Result<String, String>,
}
