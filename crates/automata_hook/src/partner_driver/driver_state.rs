use std::sync::Mutex;
use std::time::Instant;

use automata_game::autopilot::Autopilot;
use automata_game::character::Character;
use automata_game::defeat::{DefeatFlowLog, HealthLog, HoldLog};
use automata_game::exe::ExeVariant;
use automata_game::minute_line_budget::MinuteLineBudget;
use automata_game::partner::{
    BodyFields, HeldFree, HelloRule, NotFree, PartnerDrive, PartnerMachine, SendPacer,
};

use crate::game_calls::GameCalls;
use crate::game_hooks::FeedHooks;
use crate::partner_driver::body_drive::BodyDrive;
use crate::partner_driver::downed_watch::DownedDriver;
use crate::partner_driver::flight_drive::FlightDrive;

pub(super) static DRIVER: Mutex<Option<PartnerDriver>> = Mutex::new(None);
pub(super) static BODY_STATUS_LINE: Mutex<String> = Mutex::new(String::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RequestedRead {
    pub object_id: u32,
    pub phase_id: i32,
}

#[derive(Debug, Default)]
pub(super) struct StagesLogged {
    pub local_state_sent: bool,
    pub local_state_unreadable: bool,
    pub no_fresh_remote_state: bool,
    pub jump_kept_players_apart: bool,
    pub phases_that_differed: Option<(i32, i32)>,
    pub spawn_not_guarded: bool,
    pub defeat_flow_unreadable: bool,
    pub owners_health_shown: bool,
}

pub(super) struct PartnerDriver {
    pub partner_slot: u8,
    pub guarded_slot_body_handle: Option<u32>,
    pub calls: GameCalls,
    pub local_character: Character,
    pub local_exe: ExeVariant,
    pub partner_setting: Character,
    pub drive: PartnerDrive,
    pub feed: FeedHooks,
    pub starts_are_recorded: bool,
    pub spawn_is_guarded: bool,
    pub requested_read: Option<RequestedRead>,
    pub started: Instant,
    pub machine: PartnerMachine,
    pub saved_fields: Option<BodyFields>,
    pub pacer: SendPacer,
    pub hello: HelloRule,
    pub body_drive: BodyDrive,
    pub last_hero_handle: Option<u32>,
    pub autopilot: Autopilot,
    pub local_phase_id: i32,
    pub free_reason_logged: Option<Option<NotFree>>,
    pub local_not_free: Option<NotFree>,
    pub hero_health_log: HealthLog,
    pub other_player_health_log: HealthLog,
    pub defeat_flow_log: DefeatFlowLog,
    pub hold_log: HoldLog,
    pub downed: DownedDriver,
    pub free_change_lines: MinuteLineBudget,
    pub local_held_free: HeldFree,
    pub remote_held_free: HeldFree,
    pub both_held_free_logged: Option<bool>,
    pub body_reading_lines: MinuteLineBudget,
    pub body_readings_logged: u32,
    pub last_body_reading_line_ms: Option<u64>,
    pub logged: StagesLogged,
    pub flight: FlightDrive,
}
