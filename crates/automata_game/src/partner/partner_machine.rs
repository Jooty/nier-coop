use crate::character::Character;
use crate::partner::{DESTROY_RETRY_INTERVAL_MS, ENGINE_PARTNER_WAIT_MS, SPAWN_RETRY_INTERVAL_MS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeroReading {
    pub handle: u32,
    pub player_controlled: bool,
    pub computer_controlled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuddyReading {
    pub handle: u32,
    pub object_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartnerReadings {
    pub partner_connected: bool,
    pub wanted: Character,
    pub hero: Option<HeroReading>,
    /// The body in the hero's partner slot, when the slot's handle is alive.
    pub buddy: Option<BuddyReading>,
    /// None when the handle of the held body is dead.
    pub held_object_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartnerAction {
    Nothing,
    Adopt,
    DestroySlotPartner,
    Spawn {
        object_id: u32,
    },
    Drive,
    Relink,
    Release {
        restore_fields: bool,
        terminate: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartnerStep {
    pub action: PartnerAction,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeldBody {
    body_handle: u32,
    hero_handle: u32,
    made_by_mod: bool,
    relink_tried: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartnerMachine {
    held: Option<HeldBody>,
    controlled_since_ms: Option<u64>,
    wrong_partner_removed: bool,
    next_spawn_ms: u64,
    next_destroy_ms: u64,
}

impl PartnerMachine {
    pub fn held_body_handle(&self) -> Option<u32> {
        self.held.map(|held| held.body_handle)
    }

    pub fn step(&mut self, now_ms: u64, readings: &PartnerReadings) -> PartnerStep {
        let mut logs = Vec::new();
        let action = match self.held {
            Some(held) => self.step_holding(held, readings, &mut logs),
            None if readings.partner_connected => self.step_seeking(now_ms, readings, &mut logs),
            None => {
                self.start_over();
                PartnerAction::Nothing
            }
        };
        PartnerStep { action, logs }
    }

    pub fn spawned(&mut self, body_handle: u32, hero_handle: u32) -> Vec<String> {
        self.held = Some(HeldBody {
            body_handle,
            hero_handle,
            made_by_mod: true,
            relink_tried: false,
        });
        vec![format!(
            "partner: spawned a body and linked it into the hero's partner slot (handle {body_handle:#X})"
        )]
    }

    pub fn spawn_failed(&mut self, reason: &str) -> Vec<String> {
        vec![format!(
            "partner: the spawn failed ({reason}), trying again in {} s",
            SPAWN_RETRY_INTERVAL_MS / 1000
        )]
    }

    fn step_holding(
        &mut self,
        held: HeldBody,
        readings: &PartnerReadings,
        logs: &mut Vec<String>,
    ) -> PartnerAction {
        let body_alive = readings.held_object_id.is_some();
        if !readings.partner_connected {
            self.start_over();
            logs.push(
                "partner: the other player left, giving the body back to the game (putting the saved fields back is a guess)"
                    .to_owned(),
            );
            return PartnerAction::Release {
                restore_fields: body_alive,
                terminate: false,
            };
        }
        let hero_is_the_same = readings
            .hero
            .is_some_and(|hero| hero.handle == held.hero_handle);
        if !body_alive || !hero_is_the_same {
            self.start_over();
            logs.push(
                "partner: the body or the hero is gone (a load or a scene change), starting over"
                    .to_owned(),
            );
            return PartnerAction::Release {
                restore_fields: body_alive,
                terminate: false,
            };
        }
        let in_partner_slot = readings
            .buddy
            .is_some_and(|buddy| buddy.handle == held.body_handle);
        if !in_partner_slot && readings.buddy.is_none() && !held.relink_tried {
            self.held = Some(HeldBody {
                relink_tried: true,
                ..held
            });
            logs.push(
                "partner: the body fell out of the hero's partner slot (the engine may have dismissed it), linking it again"
                    .to_owned(),
            );
            return PartnerAction::Relink;
        }
        if !in_partner_slot {
            self.start_over();
            logs.push(
                "partner: the body is not in the hero's partner slot and linking it again did not help, starting over"
                    .to_owned(),
            );
            return PartnerAction::Release {
                restore_fields: !held.made_by_mod,
                terminate: held.made_by_mod,
            };
        }
        if readings.held_object_id != Some(readings.wanted.object_id()) {
            self.start_over();
            logs.push(format!(
                "partner: the held body is not {}, letting it go to replace it",
                readings.wanted.launcher_id()
            ));
            return PartnerAction::Release {
                restore_fields: true,
                terminate: false,
            };
        }
        if held.relink_tried {
            self.held = Some(HeldBody {
                relink_tried: false,
                ..held
            });
        }
        PartnerAction::Drive
    }

    fn step_seeking(
        &mut self,
        now_ms: u64,
        readings: &PartnerReadings,
        logs: &mut Vec<String>,
    ) -> PartnerAction {
        let Some(hero) = readings
            .hero
            .filter(|hero| hero.player_controlled && !hero.computer_controlled)
        else {
            self.controlled_since_ms = None;
            return PartnerAction::Nothing;
        };
        let wanted = readings.wanted;
        let since_ms = match self.controlled_since_ms {
            Some(since_ms) => since_ms,
            None => {
                logs.push(format!(
                    "partner: a controlled body exists, waiting up to {} s for the engine to make {}",
                    ENGINE_PARTNER_WAIT_MS / 1000,
                    wanted.launcher_id()
                ));
                self.controlled_since_ms = Some(now_ms);
                now_ms
            }
        };
        match readings.buddy {
            Some(buddy) if buddy.object_id == wanted.object_id() => {
                self.held = Some(HeldBody {
                    body_handle: buddy.handle,
                    hero_handle: hero.handle,
                    made_by_mod: false,
                    relink_tried: false,
                });
                logs.push(format!(
                    "partner: taking over the partner the engine made ({}, handle {:#X})",
                    wanted.launcher_id(),
                    buddy.handle
                ));
                PartnerAction::Adopt
            }
            Some(buddy) => {
                if now_ms < self.next_destroy_ms {
                    return PartnerAction::Nothing;
                }
                self.next_destroy_ms = now_ms + DESTROY_RETRY_INTERVAL_MS;
                self.wrong_partner_removed = true;
                logs.push(format!(
                    "partner: the partner slot holds object {:#X}, not {}; removing it",
                    buddy.object_id,
                    wanted.launcher_id()
                ));
                PartnerAction::DestroySlotPartner
            }
            None => {
                let waited_long_enough = now_ms.saturating_sub(since_ms) >= ENGINE_PARTNER_WAIT_MS;
                if !(waited_long_enough || self.wrong_partner_removed)
                    || now_ms < self.next_spawn_ms
                {
                    return PartnerAction::Nothing;
                }
                self.next_spawn_ms = now_ms + SPAWN_RETRY_INTERVAL_MS;
                let reason = if self.wrong_partner_removed {
                    "the slot is empty after the removal"
                } else {
                    "the engine made no partner"
                };
                logs.push(format!(
                    "partner: {reason}, spawning {} (object {:#X})",
                    wanted.launcher_id(),
                    wanted.object_id()
                ));
                PartnerAction::Spawn {
                    object_id: wanted.object_id(),
                }
            }
        }
    }

    fn start_over(&mut self) {
        *self = Self::default();
    }
}
