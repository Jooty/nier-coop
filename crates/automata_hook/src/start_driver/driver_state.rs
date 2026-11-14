use std::sync::Mutex;
use std::time::Instant;

use automata_game::load_gate::{BootScreenKeep, LoadGate, LoadWatch, LoadingScreenReading};
use automata_game::partner::{PartnerDataWait, ReadyLatch};
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
    pub slot_calls_verified: bool,
    pub in_play: bool,
    pub ready: ReadyLatch,
    pub gate: LoadGate,
    pub boot_screen: BootScreenKeep,
    pub partner_data: PartnerDataWait,
    pub state_without_body_sent_at_ms: Option<u64>,
    pub scene_left_while_arriving: bool,
    pub load_end_logged: bool,
    pub loading_screen_logged: Option<LoadingScreenReading>,
    pub loading_screen_lines: u32,
    pub load_watch: LoadWatch,
    pub campaign_loads_hooked: bool,
    pub hold_waiting_since_ms: Option<u64>,
    pub hold_waiting_logged_at_ms: Option<u64>,
}
