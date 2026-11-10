use std::sync::Mutex;
use std::time::Instant;

use automata_game::travel::{LoadedPoints, TravelMachine, TravelView};

use crate::game_calls::GameCalls;

pub(super) static DRIVER: Mutex<Option<TravelDriver>> = Mutex::new(None);
pub(super) static VIEW: Mutex<Option<TravelView>> = Mutex::new(None);

#[derive(Debug, Default)]
pub(super) struct FirstUsesLogged {
    pub hero_moved: bool,
    pub transport_called: bool,
}

pub(super) struct TravelDriver {
    pub calls: GameCalls,
    pub started: Instant,
    pub machine: TravelMachine,
    pub loaded: LoadedPoints,
    pub points_read_at_ms: Option<u64>,
    pub logged_point_names: Vec<String>,
    pub move_line: String,
    pub logged: FirstUsesLogged,
}
