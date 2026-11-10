use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Instant;

use automata_game::character::Character;
use automata_game::partner::{
    BodyFields, HelloRule, PartnerDrive, PartnerMachine, PositionCorrector, SendPacer, StateRule,
};

use crate::game_calls::GameCalls;

pub(super) static DRIVER: Mutex<Option<PartnerDriver>> = Mutex::new(None);

#[derive(Debug, Default)]
pub(super) struct StagesLogged {
    pub local_state_sent: bool,
    pub local_state_unreadable: bool,
    pub fields_held: bool,
    pub still_computer_controlled: bool,
    pub body_not_updating: bool,
    pub no_fresh_remote_state: bool,
    pub position_nudged: bool,
    pub teleported: bool,
    pub facing_written: bool,
    pub state_started: bool,
    pub other_vtable: bool,
    pub blocked_states: HashSet<u16>,
}

pub(super) struct PartnerDriver {
    pub calls: GameCalls,
    pub local_character: Character,
    pub partner_setting: Character,
    pub drive: PartnerDrive,
    pub started: Instant,
    pub machine: PartnerMachine,
    pub saved_fields: Option<BodyFields>,
    pub pacer: SendPacer,
    pub hello: HelloRule,
    pub state_rule: StateRule,
    pub corrector: PositionCorrector,
    pub last_remote_sequence: Option<u32>,
    pub logged: StagesLogged,
}
