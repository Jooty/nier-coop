use std::sync::Mutex;
use std::time::Instant;

use automata_game::enemies::{Census, EnemySighting, MismatchWatch, RosterAssembly, RosterPacer};

use crate::game_calls::GameCalls;

pub(super) static DRIVER: Mutex<Option<EnemiesDriver>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct CensusSummary {
    pub lists: usize,
    pub enemies: usize,
    pub with_the_base_damage_code: usize,
    pub without_the_entry_id_on_the_body: usize,
}

pub(super) struct EnemiesDriver {
    pub calls: GameCalls,
    pub started: Instant,
    pub last_census_ms: Option<u64>,
    pub census: Census,
    pub pacer: RosterPacer,
    pub assembly: RosterAssembly,
    pub other_roster: Option<Vec<EnemySighting>>,
    pub mismatches: MismatchWatch,
    pub mismatch_lines_left: u32,
    pub was_connected: bool,
    pub summary_logged: Option<(CensusSummary, u64)>,
}
