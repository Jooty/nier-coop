use crate::character::Character;
use crate::mode::SessionMode;
use crate::partner::{MAX_HELD_BODIES, partner_is_in_play};

pub const MAX_EXTRA_BODIES: usize = MAX_HELD_BODIES - 1;

/// One other player of the session as the roster sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OtherPlayer {
    pub slot: u8,
    pub character: Option<Character>,
    /// The player's own game says it is in a scene with a body.
    pub in_play: bool,
    pub newest_state_age_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RosterReadings<'a> {
    pub mode: SessionMode,
    pub scene_in_play: bool,
    pub phase_id: i32,
    pub others: &'a [OtherPlayer],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtraBodyStep {
    Nothing,
    RequestData(Character),
    Spawn(Character),
    Drive(u32),
    /// Terminate the body: its player left, went quiet or wears another character now.
    Remove(u32),
    /// The scene the body was made in is gone; release it from the guards, never terminate it.
    Forget(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotStep {
    pub slot: u8,
    pub step: ExtraBodyStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExtraBody {
    slot: u8,
    character: Character,
    handle: u32,
    phase_id: i32,
}

/// The bodies made for players beyond the partner, outside the companion slot.
#[derive(Debug, Clone, Default)]
pub struct ExtraBodyRoster {
    bodies: Vec<ExtraBody>,
}

impl ExtraBodyRoster {
    /// What to do this tick, one step per slot that has a body or wants one.
    pub fn steps(
        &self,
        readings: &RosterReadings<'_>,
        data_is_loaded: impl Fn(Character) -> bool,
    ) -> Vec<SlotStep> {
        if !readings.scene_in_play {
            return self.every_body_as(ExtraBodyStep::Forget);
        }
        let mut steps = Vec::new();
        for body in &self.bodies {
            let step = if body.phase_id != readings.phase_id {
                ExtraBodyStep::Forget(body.handle)
            } else if further_players(readings).any(|player| keeps_its_body(player, body)) {
                ExtraBodyStep::Drive(body.handle)
            } else {
                ExtraBodyStep::Remove(body.handle)
            };
            steps.push(SlotStep {
                slot: body.slot,
                step,
            });
        }
        let mut bodies_wanted = self.bodies.len();
        for player in further_players(readings) {
            if self.bodies.iter().any(|body| body.slot == player.slot) {
                continue;
            }
            let step = match player.character {
                Some(character) if is_present(player) && bodies_wanted < MAX_EXTRA_BODIES => {
                    bodies_wanted += 1;
                    if data_is_loaded(character) {
                        ExtraBodyStep::Spawn(character)
                    } else {
                        ExtraBodyStep::RequestData(character)
                    }
                }
                _ => ExtraBodyStep::Nothing,
            };
            steps.push(SlotStep {
                slot: player.slot,
                step,
            });
        }
        steps
    }

    pub fn made(&mut self, slot: u8, character: Character, handle: u32, phase_id: i32) {
        self.gone(slot);
        self.bodies.push(ExtraBody {
            slot,
            character,
            handle,
            phase_id,
        });
    }

    /// The body of the slot was removed or forgotten.
    pub fn gone(&mut self, slot: u8) {
        self.bodies.retain(|body| body.slot != slot);
    }

    pub fn handle_of(&self, slot: u8) -> Option<u32> {
        self.bodies
            .iter()
            .find(|body| body.slot == slot)
            .map(|body| body.handle)
    }

    pub fn len(&self) -> usize {
        self.bodies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }

    fn every_body_as(&self, step: fn(u32) -> ExtraBodyStep) -> Vec<SlotStep> {
        self.bodies
            .iter()
            .map(|body| SlotStep {
                slot: body.slot,
                step: step(body.handle),
            })
            .collect()
    }
}

fn further_players<'a>(readings: &RosterReadings<'a>) -> impl Iterator<Item = &'a OtherPlayer> {
    let others = readings.others;
    let partner_slot = others.iter().map(|player| player.slot).min();
    let free_roam = readings.mode == SessionMode::FreeRoam;
    // The lowest other slot is the partner of the existing partner machine.
    others
        .iter()
        .filter(move |player| free_roam && Some(player.slot) != partner_slot)
}

fn is_present(player: &OtherPlayer) -> bool {
    player.in_play && partner_is_in_play(true, player.newest_state_age_ms)
}

fn keeps_its_body(player: &OtherPlayer, body: &ExtraBody) -> bool {
    player.slot == body.slot && is_present(player) && player.character == Some(body.character)
}
