use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Instant;

use automata_game::autopilot::Autopilot;
use automata_game::character::Character;
use automata_game::exe::ExeVariant;
use automata_game::partner::{
    BodyFields, HelloRule, NotFree, PartnerDrive, PartnerMachine, PositionCorrector, SendPacer,
    StateRule,
};

use crate::game_calls::GameCalls;
use crate::game_hooks::FeedHooks;

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
    pub states_not_started: HashSet<u16>,
    pub body_states_left_alone: HashSet<i32>,
    pub phases_that_differed: Option<(i32, i32)>,
}

pub(super) struct PartnerDriver {
    pub calls: GameCalls,
    pub local_character: Character,
    pub local_exe: ExeVariant,
    pub partner_setting: Character,
    pub drive: PartnerDrive,
    pub feed: FeedHooks,
    pub starts_are_recorded: bool,
    pub started: Instant,
    pub machine: PartnerMachine,
    pub saved_fields: Option<BodyFields>,
    pub pacer: SendPacer,
    pub hello: HelloRule,
    pub state_rule: StateRule,
    pub corrector: PositionCorrector,
    pub last_remote_sequence: Option<u32>,
    pub last_hero_handle: Option<u32>,
    pub autopilot: Autopilot,
    pub local_phase_id: i32,
    pub free_reason_logged: Option<Option<NotFree>>,
    pub free_change_lines: u32,
    pub logged: StagesLogged,
}
