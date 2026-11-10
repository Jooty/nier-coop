use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::time::Instant;

use automata_game::travel::{LoadedPoints, TravelMachine, TravelView};

use crate::game_calls::GameCalls;

pub(super) static DRIVER: Mutex<Option<TravelDriver>> = Mutex::new(None);
pub(super) static VIEW: Mutex<Option<TravelView>> = Mutex::new(None);
pub(super) static VIEW_ASKED_AT_MS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Default)]
pub(super) struct FirstUsesLogged {
    pub hero_moved: bool,
    pub transport_called: bool,
    pub older_requests_ignored: bool,
    pub ignored_lines: HashSet<String>,
}

pub(super) struct TravelDriver {
    pub calls: GameCalls,
    pub started: Instant,
    pub machine: TravelMachine,
    pub loaded: LoadedPoints,
    pub points_read_at_ms: Option<u64>,
    pub logged_point_names: Vec<String>,
    pub move_line: String,
    pub move_line_logged_at_ms: Option<u64>,
    pub moved_at_ms: Option<u64>,
    pub logged: FirstUsesLogged,
}
