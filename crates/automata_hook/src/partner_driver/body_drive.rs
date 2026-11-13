use std::collections::HashSet;

use automata_game::partner::{
    BODY_BREAK_FIELD_COUNT, BODY_BREAK_FIELDS_OFFSET, BODY_FACING_OFFSET,
    BODY_PLAYER_CONTROLLED_OFFSET, BODY_POSITION_OFFSET, BODY_ROTATION_EULER_OFFSET,
    BODY_STATE_OFFSET, BODY_SUSPEND_FLAGS_OFFSET, BODY_UPDATE_GATE_OFFSET,
    POSITION_ERROR_TELEPORTED, PartnerDrive, PositionCorrection, PositionCorrector, RemoteState,
    SUSPENDED_FLAG_BIT, StateDecision, StateRule, facing_needs_write,
};
use automata_game::partner_flight::rider_moved_without_teleport_line;
use automata_proto::PlayerState;
use nc_hook::diagnostics::log_line;

use crate::game_calls::{GameCalls, Vec4};

#[derive(Debug, Default)]
struct BodyDriveLogged {
    fields_held: bool,
    still_computer_controlled: bool,
    body_not_updating: bool,
    position_nudged: bool,
    teleported: bool,
    facing_written: bool,
    state_started: bool,
    other_vtable: bool,
    states_not_started: HashSet<u16>,
    body_states_left_alone: HashSet<i32>,
    downed_body_moved_without_teleport: bool,
    riding_body_moved_without_teleport: bool,
}

/// Puts another player's state on a body this game holds: its control fields, position, facing
/// and state starts. Every log line starts with the owner's name.
pub(crate) struct BodyDrive {
    owner: String,
    state_rule: StateRule,
    corrector: PositionCorrector,
    last_remote_sequence: Option<u32>,
    logged: BodyDriveLogged,
    body_rides: bool,
}

impl BodyDrive {
    pub(crate) fn new(owner: impl Into<String>, drive: PartnerDrive) -> Self {
        Self {
            owner: owner.into(),
            state_rule: StateRule::new(drive),
            corrector: PositionCorrector::default(),
            last_remote_sequence: None,
            logged: BodyDriveLogged::default(),
            body_rides: false,
        }
    }

    /// Starts over on a newly held body; what was logged once stays logged.
    pub(crate) fn restart(&mut self, drive: PartnerDrive) {
        self.state_rule = StateRule::new(drive);
        self.corrector = PositionCorrector::default();
        self.last_remote_sequence = None;
        self.body_rides = false;
    }

    /// A riding body is never moved by the game's teleport, which would put it in its default
    /// state; it is moved by plain position writes instead.
    pub(crate) fn set_body_rides(&mut self, rides: bool) {
        self.body_rides = rides;
    }

    pub(crate) fn hold_control_fields(&mut self, calls: &GameCalls, body: usize) -> bool {
        let Ok(current) = calls.read_body_fields(body) else {
            return false;
        };
        let wanted = current.taken_over();
        if calls.write_body_fields(body, &current, &wanted).is_err() {
            return false;
        }
        if !self.logged.fields_held {
            self.logged.fields_held = true;
            log_line(&format!(
                "{}: holding the body's control fields every tick (flags {:#X} to {:#X}, player flag 1, input sources off)",
                self.owner, current.flags, wanted.flags
            ));
        }
        if wanted.is_computer_controlled() && !self.logged.still_computer_controlled {
            self.logged.still_computer_controlled = true;
            log_line(&format!(
                "{}: the body still counts as computer controlled (flags {:#X}); bit 6 is a guess and is left alone, so its controller will not run",
                self.owner, wanted.flags
            ));
        }
        true
    }

    pub(crate) fn body_is_updating(&mut self, calls: &GameCalls, body: usize) -> bool {
        let suspend_flags = calls.read_u32_at(calls.body_field(body, BODY_SUSPEND_FLAGS_OFFSET));
        let update_gate = calls.read_i32_at(calls.body_field(body, BODY_UPDATE_GATE_OFFSET));
        let updating = suspend_flags.is_ok_and(|flags| flags & SUSPENDED_FLAG_BIT == 0)
            && update_gate.is_ok_and(|gate| gate < 0);
        if !updating && !self.logged.body_not_updating {
            self.logged.body_not_updating = true;
            log_line(&format!(
                "{}: the body is suspended or its update gate is closed (suspend flags {suspend_flags:X?}, gate {update_gate:?}); nothing is corrected while that holds",
                self.owner
            ));
        }
        updating
    }

    /// Corrects position and facing toward the other player's state, and starts its state on
    /// the body once per new message.
    pub(crate) fn apply_remote(
        &mut self,
        calls: &GameCalls,
        body: usize,
        now_ms: u64,
        remote: &PlayerState,
        body_lies_downed: bool,
    ) {
        self.correct_position_and_facing(calls, body, now_ms, remote, body_lies_downed);
        if self.last_remote_sequence != Some(remote.sequence) {
            self.last_remote_sequence = Some(remote.sequence);
            self.apply_state_rule(calls, body, remote);
        }
    }

    fn correct_position_and_facing(
        &mut self,
        calls: &GameCalls,
        body: usize,
        now_ms: u64,
        remote: &PlayerState,
        body_lies_downed: bool,
    ) {
        let Ok(local) = calls.read_body_position(body) else {
            return;
        };
        let local_position = [local[0], local[1], local[2]];
        match self
            .corrector
            .correct(now_ms, local_position, remote.position)
        {
            PositionCorrection::Leave => {}
            PositionCorrection::Nudge(position) => {
                let written = calls
                    .write_f32s_at(calls.body_field(body, BODY_POSITION_OFFSET), &position)
                    .is_ok();
                if written && !self.logged.position_nudged {
                    self.logged.position_nudged = true;
                    log_line(&format!(
                        "{}: first position nudge toward the other player (physics may undo plain writes: a guess)",
                        self.owner
                    ));
                }
            }
            PositionCorrection::Teleport if self.body_rides => {
                let written = calls
                    .write_f32s_at(
                        calls.body_field(body, BODY_POSITION_OFFSET),
                        &remote.position,
                    )
                    .is_ok();
                if written && !self.logged.riding_body_moved_without_teleport {
                    self.logged.riding_body_moved_without_teleport = true;
                    log_line(&rider_moved_without_teleport_line());
                }
            }
            PositionCorrection::Teleport if body_lies_downed => {
                // The game's teleport would put the body in its default pose: a downed body is
                // moved by a plain write, so it stays lying.
                let written = calls
                    .write_f32s_at(
                        calls.body_field(body, BODY_POSITION_OFFSET),
                        &remote.position,
                    )
                    .is_ok();
                if written && !self.logged.downed_body_moved_without_teleport {
                    self.logged.downed_body_moved_without_teleport = true;
                    log_line(
                        "downed: the downed other player's body was more than a teleport's distance from its owner; moved by a plain position write, not the game's teleport, so its pose is kept (logged once)",
                    );
                }
            }
            PositionCorrection::Teleport => {
                self.teleport(calls, body, local[3], remote);
                return;
            }
        }
        let facing_address = calls.body_field(body, BODY_FACING_OFFSET);
        let needs_write = calls
            .read_f32_at(facing_address)
            .is_ok_and(|facing| facing_needs_write(facing, remote.body_facing));
        if needs_write
            && calls
                .write_f32_at(facing_address, remote.body_facing)
                .is_ok()
            && !self.logged.facing_written
        {
            self.logged.facing_written = true;
            log_line(&format!(
                "{}: first body facing write (the angle error was large)",
                self.owner
            ));
        }
    }

    fn teleport(&mut self, calls: &GameCalls, body: usize, position_w: f32, remote: &PlayerState) {
        let Ok(rotation) =
            calls.read_f32s_at::<4>(calls.body_field(body, BODY_ROTATION_EULER_OFFSET))
        else {
            return;
        };
        let position = Vec4([
            remote.position[0],
            remote.position[1],
            remote.position[2],
            position_w,
        ]);
        // Zero x and z keep the game on its branch that stores the vector as the body's rotation.
        let rotation = Vec4([0.0, remote.body_facing, 0.0, rotation[3]]);
        if !teleport_without_the_player_flag(calls, body, &position, &rotation) {
            return;
        }
        // The game's teleport put the body in its default state, and the flag goes back to 1.
        self.state_rule.forget_started();
        self.hold_control_fields(calls, body);
        if !self.logged.teleported {
            self.logged.teleported = true;
            log_line(&format!(
                "{}: first teleport of the body to the other player (the error was above {POSITION_ERROR_TELEPORTED})",
                self.owner
            ));
        }
    }

    #[expect(unsafe_code, reason = "starts a state on the held body")]
    fn apply_state_rule(&mut self, calls: &GameCalls, body: usize, remote: &PlayerState) {
        let Ok(local_state) = calls.read_i32_at(calls.body_field(body, BODY_STATE_OFFSET)) else {
            return;
        };
        let remote_state = RemoteState {
            state: remote.state,
            last_start: remote.last_start,
        };
        match self.state_rule.observe(local_state, remote_state) {
            StateDecision::Nothing => {}
            StateDecision::BodyNotInAPlainState(state) => {
                if self.logged.body_states_left_alone.insert(state) {
                    let break_fields = calls.read_u32s_at::<BODY_BREAK_FIELD_COUNT>(
                        calls.body_field(body, BODY_BREAK_FIELDS_OFFSET),
                    );
                    log_line(&format!(
                        "{}: the body is in state {state:#X}, not one of the plain states; no state is started on it while that holds (the other player is in {:#X}; the body's four break fields read {break_fields:X?})",
                        self.owner, remote.state
                    ));
                }
            }
            StateDecision::NotAllowed(state) => {
                if self.logged.states_not_started.insert(state) {
                    log_line(&format!(
                        "{}: the other player is in state {state:#X}, which is not on the list of states started on the body; only position correction carries the body through it",
                        self.owner
                    ));
                }
            }
            StateDecision::Start(ids) => {
                // SAFETY: the body was resolved from its live handle this tick and the rule
                // hands out only states on the allow list.
                let started = unsafe { calls.start_state(body, ids) };
                if started && !self.logged.state_started {
                    self.logged.state_started = true;
                    log_line(&format!(
                        "{}: first state start on the body (state {:#X}, arguments {}, {}, {}: {}; the body was in {local_state:#X})",
                        self.owner,
                        ids.state,
                        ids.variant,
                        ids.b,
                        ids.c,
                        if remote.last_start.is_some() {
                            "the other player's own start call"
                        } else {
                            "phase 0, the other player records no starts"
                        }
                    ));
                }
                if !started && !self.logged.other_vtable {
                    self.logged.other_vtable = true;
                    log_line(&format!(
                        "{}: the body's vtable is not the player body's, no state is started on it",
                        self.owner
                    ));
                }
            }
        }
    }
}

/// Calls the game's teleport on a held body with its player flag cleared first; false when the
/// flag could not be written.
#[expect(unsafe_code, reason = "calls the game's teleport on a held body")]
pub(crate) fn teleport_without_the_player_flag(
    calls: &GameCalls,
    body: usize,
    position: &Vec4,
    rotation: &Vec4,
) -> bool {
    // With the player flag set the game's teleport also runs its "the hero was moved" part.
    let flag_is_clear = calls
        .write_i32_at(calls.body_field(body, BODY_PLAYER_CONTROLLED_OFFSET), 0)
        .is_ok();
    if !flag_is_clear {
        return false;
    }
    // SAFETY: the caller resolved the body from its live handle or spawned it this tick.
    unsafe { calls.teleport_reset_hap(body, position, rotation) };
    true
}
