use automata_game::travel::{TravelPoint, TravelView};

/// What the Teleport section reads and asks; the game side fills it in on Windows.
#[derive(Debug, Clone, Copy)]
pub struct TeleportHooks {
    pub view: fn() -> TravelView,
    pub ask_jump: fn(TravelPoint) -> Result<(), String>,
    pub ask_go_to_partner: fn() -> Result<String, String>,
    pub ask_bring_partner: fn() -> Result<String, String>,
}
