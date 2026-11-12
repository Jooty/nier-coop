use crate::character::Character;
use crate::partner::{DESTROY_RETRY_INTERVAL_MS, ENGINE_PARTNER_WAIT_MS, SPAWN_RETRY_INTERVAL_MS};

pub const LOCAL_NOT_FREE_HOLD_BEFORE_RELEASE_MS: u64 = 1000;

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
    /// The other player's game is sending states, not only its launcher connected.
    pub partner_in_play: bool,
    /// No cutscene, event or scene change holds the local player or the other player.
    pub both_free: bool,
    /// The two players' free flags before the hold-for-a-second rule; they end a hold the moment both are set.
    pub local_free_now: bool,
    pub other_free_now: bool,
    pub wanted: Character,
    pub hero: Option<HeroReading>,
    /// The body in the hero's partner slot, when the slot's handle is alive.
    pub buddy: Option<BuddyReading>,
    /// The handle in the hero's partner slot when it is alive and not a player body.
    pub other_slot_holder: Option<u32>,
    /// None when the handle of the held body is dead.
    pub held_object_id: Option<u32>,
    /// The wanted character's object data and all it depends on are read: a spawn waits for nothing.
    pub wanted_data_is_loaded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartnerAction {
    Nothing,
    Adopt,
    RequestObjectRead {
        object_id: u32,
    },
    DestroySlotPartner,
    Spawn {
        object_id: u32,
    },
    Drive,
    Hold,
    Relink,
    Release {
        restore_fields: bool,
        terminate: bool,
        made_by_mod: bool,
        mark_dismissed: bool,
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
    stands_in_until_the_wanted_data_is_loaded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NotFreeHold {
    since_ms: u64,
    local_not_free_since_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartnerMachine {
    held: Option<HeldBody>,
    not_free_hold: Option<NotFreeHold>,
    controlled_since_ms: Option<u64>,
    wrong_partner_removed: bool,
    read_asked_for_object_id: Option<u32>,
    empty_slot_wait_for_data_logged: bool,
    other_slot_holder_logged: Option<u32>,
    next_spawn_ms: u64,
    next_destroy_ms: u64,
}

impl PartnerMachine {
    pub fn held_body_handle(&self) -> Option<u32> {
        self.held.map(|held| held.body_handle)
    }

    pub fn step(&mut self, now_ms: u64, readings: &PartnerReadings) -> PartnerStep {
        let mut logs = Vec::new();
        let hold_before = self.not_free_hold;
        let action = match self.held {
            Some(held) => self.step_holding(now_ms, held, readings, &mut logs),
            None if readings.partner_in_play => self.step_seeking(now_ms, readings, &mut logs),
            None => {
                self.start_over();
                PartnerAction::Nothing
            }
        };
        if let Some(hold) = hold_before
            && self.not_free_hold.is_none()
        {
            let ended_in = match action {
                PartnerAction::Drive => "driving",
                PartnerAction::Release { .. } => "a release",
                _ => "another step",
            };
            logs.push(format!(
                "partner: the hold of the body ended after {} ms, in {ended_in}",
                now_ms.saturating_sub(hold.since_ms)
            ));
        }
        PartnerStep { action, logs }
    }

    pub fn spawned(&mut self, body_handle: u32, hero_handle: u32) -> Vec<String> {
        self.held = Some(HeldBody {
            body_handle,
            hero_handle,
            made_by_mod: true,
            relink_tried: false,
            stands_in_until_the_wanted_data_is_loaded: false,
        });
        vec![format!(
            "partner: spawned a body and linked it into the hero's partner slot (handle {body_handle:#X})"
        )]
    }

    pub fn spawn_failed(&mut self, reason: &str) -> Vec<String> {
        // A body that could not be made may lack data again: the next step asks for the read anew.
        self.read_asked_for_object_id = None;
        vec![format!(
            "partner: the spawn failed ({reason}), trying again in {} s",
            SPAWN_RETRY_INTERVAL_MS / 1000
        )]
    }

    fn step_holding(
        &mut self,
        now_ms: u64,
        held: HeldBody,
        readings: &PartnerReadings,
        logs: &mut Vec<String>,
    ) -> PartnerAction {
        let body_alive = readings.held_object_id.is_some();
        if !readings.partner_in_play {
            self.start_over();
            logs.push(
                "partner: the other player left or its game sends no states, giving the body back to the game (putting the saved fields back is a guess)"
                    .to_owned(),
            );
            return PartnerAction::Release {
                restore_fields: body_alive,
                terminate: false,
                made_by_mod: held.made_by_mod,
                mark_dismissed: false,
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
                made_by_mod: held.made_by_mod,
                mark_dismissed: false,
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
            // The engine's own dismissal of an adopted body can only finish with its mark set.
            return PartnerAction::Release {
                restore_fields: !held.made_by_mod,
                terminate: held.made_by_mod,
                made_by_mod: held.made_by_mod,
                mark_dismissed: !held.made_by_mod,
            };
        }
        let is_the_wanted_character = readings.held_object_id == Some(readings.wanted.object_id());
        let stays_as_stand_in =
            held.stands_in_until_the_wanted_data_is_loaded && !readings.wanted_data_is_loaded;
        if !is_the_wanted_character && !stays_as_stand_in {
            self.start_over();
            logs.push(format!(
                "partner: the held body is not {}, letting it go to replace it",
                readings.wanted.launcher_id()
            ));
            return PartnerAction::Release {
                restore_fields: true,
                terminate: false,
                made_by_mod: held.made_by_mod,
                mark_dismissed: false,
            };
        }
        if let Some(action) = self.hold_while_a_player_is_not_free(now_ms, held, readings, logs) {
            return action;
        }
        if held.relink_tried {
            self.held = Some(HeldBody {
                relink_tried: false,
                ..held
            });
        }
        PartnerAction::Drive
    }

    fn hold_while_a_player_is_not_free(
        &mut self,
        now_ms: u64,
        held: HeldBody,
        readings: &PartnerReadings,
        logs: &mut Vec<String>,
    ) -> Option<PartnerAction> {
        let free_now = readings.local_free_now && readings.other_free_now;
        let Some(hold) = self.not_free_hold else {
            if readings.both_free {
                return None;
            }
            let local_player_is_the_cause = !readings.local_free_now;
            self.not_free_hold = Some(NotFreeHold {
                since_ms: now_ms,
                local_not_free_since_ms: local_player_is_the_cause.then_some(now_ms),
            });
            logs.push(format!(
                "partner: {} not free, keeping the body and holding it still (the local player's game gives it up after {} ms)",
                if local_player_is_the_cause {
                    "the local player is"
                } else {
                    "the other player is"
                },
                LOCAL_NOT_FREE_HOLD_BEFORE_RELEASE_MS
            ));
            return Some(PartnerAction::Hold);
        };
        if free_now {
            self.not_free_hold = None;
            return None;
        }
        let local_not_free_since_ms = if readings.local_free_now {
            None
        } else {
            Some(hold.local_not_free_since_ms.unwrap_or(now_ms))
        };
        self.not_free_hold = Some(NotFreeHold {
            local_not_free_since_ms,
            ..hold
        });
        if local_not_free_since_ms.is_some_and(|since_ms| {
            now_ms.saturating_sub(since_ms) > LOCAL_NOT_FREE_HOLD_BEFORE_RELEASE_MS
        }) {
            self.start_over();
            logs.push(
                "partner: a cutscene, an event or a scene change holds the local player, giving the body back to the game until both are free"
                    .to_owned(),
            );
            return Some(PartnerAction::Release {
                restore_fields: true,
                terminate: false,
                made_by_mod: held.made_by_mod,
                mark_dismissed: false,
            });
        }
        Some(PartnerAction::Hold)
    }

    fn step_seeking(
        &mut self,
        now_ms: u64,
        readings: &PartnerReadings,
        logs: &mut Vec<String>,
    ) -> PartnerAction {
        if !readings.both_free {
            // The wait for the engine's own partner counts from the moment both players are free.
            self.controlled_since_ms = None;
            return PartnerAction::Nothing;
        }
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
                    stands_in_until_the_wanted_data_is_loaded: false,
                });
                logs.push(format!(
                    "partner: taking over the partner the engine made ({}, handle {:#X})",
                    wanted.launcher_id(),
                    buddy.handle
                ));
                PartnerAction::Adopt
            }
            Some(_) | None
                if !readings.wanted_data_is_loaded
                    && readings.other_slot_holder.is_none()
                    && self.read_asked_for_object_id != Some(wanted.object_id()) =>
            {
                self.read_asked_for_object_id = Some(wanted.object_id());
                logs.push(format!(
                    "partner: the data of {} (object {:#X}) is not loaded in this place, asking the game to read it; nothing is spawned before it is there",
                    wanted.launcher_id(),
                    wanted.object_id()
                ));
                PartnerAction::RequestObjectRead {
                    object_id: wanted.object_id(),
                }
            }
            Some(buddy) if !readings.wanted_data_is_loaded => {
                self.held = Some(HeldBody {
                    body_handle: buddy.handle,
                    hero_handle: hero.handle,
                    made_by_mod: false,
                    relink_tried: false,
                    stands_in_until_the_wanted_data_is_loaded: true,
                });
                logs.push(format!(
                    "partner: keeping the partner the engine made (object {:#X}, handle {:#X}) and driving it in place of {} until the data of {} is loaded",
                    buddy.object_id,
                    buddy.handle,
                    wanted.launcher_id(),
                    wanted.launcher_id()
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
            None if readings.other_slot_holder.is_some() => {
                if self.other_slot_holder_logged != readings.other_slot_holder {
                    self.other_slot_holder_logged = readings.other_slot_holder;
                    logs.push(format!(
                        "partner: the hero's partner slot holds something that is not a player body (handle {:#X}); nothing is spawned while it is there",
                        readings.other_slot_holder.unwrap_or_default()
                    ));
                }
                PartnerAction::Nothing
            }
            None if !readings.wanted_data_is_loaded => {
                if !self.empty_slot_wait_for_data_logged {
                    self.empty_slot_wait_for_data_logged = true;
                    logs.push(format!(
                        "partner: the partner slot is empty and the data of {} is not loaded yet, so there is no partner body until the game has read it",
                        wanted.launcher_id()
                    ));
                }
                PartnerAction::Nothing
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
