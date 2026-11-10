use std::sync::Mutex;
use std::time::Instant;

use automata_game::start::{StartCall, StartMachine};

use crate::game_calls::GameCalls;

pub(super) static DRIVER: Mutex<Option<Driver>> = Mutex::new(None);
pub(super) static STATUS_LINE: Mutex<String> = Mutex::new(String::new());

pub(super) struct Driver {
    pub machine: StartMachine,
    pub calls: GameCalls,
    pub started: Instant,
    pub phase_name_buffer: Vec<u8>,
    pub after_phase_start: Vec<StartCall>,
    pub slot: u32,
}
