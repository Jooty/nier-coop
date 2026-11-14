use crate::rule_rooms::{RULE_ROOMS, RuleAct, RuleRoom, RuleRoomCall, rule_room_call};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulePlayer {
    Host,
    Guest,
}

/// Where this game's current rule room stands. `AskedOfTheHost` is the guest's broken rule waiting
/// for the host's run, or on the host the guest's break it took and queued but has not run yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleState {
    Armed,
    AskedOfTheHost,
    Broken,
    Cleared,
}

/// One popped function, as the rule rooms see it. `shared`: both players play the same phase of a
/// connected session; `own_fire`: this game's own callback fired it, not a copy queued for it;
/// `queued_by_the_mod`: the mod queued it here, as the partner's run or a held call let go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RulePopFacts<'a> {
    pub pack: &'a str,
    pub function: &'a str,
    pub player: RulePlayer,
    pub shared: bool,
    pub own_fire: bool,
    pub queued_by_the_mod: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulePop {
    NotARuleRoomCall,
    PassesOn,
    GuestStartRunsHereOnly {
        room: &'static RuleRoom,
    },
    GuestTakenIn {
        room: &'static RuleRoom,
    },
    BrokenAlone {
        room: &'static RuleRoom,
        act: RuleAct,
    },
    HostBreaks {
        room: &'static RuleRoom,
        act: RuleAct,
    },
    GuestAsksTheHost {
        room: &'static RuleRoom,
        act: RuleAct,
    },
    HostsBreakRunsHere {
        room: &'static RuleRoom,
        act: RuleAct,
    },
    GuestsBreakRunsHere {
        room: &'static RuleRoom,
        act: RuleAct,
    },
    DropAlreadyBroken {
        room: &'static RuleRoom,
        act: RuleAct,
        player: RulePlayer,
    },
    DropGuestTimeUp {
        room: &'static RuleRoom,
    },
    DropGuestRestart {
        room: &'static RuleRoom,
    },
    DropGuestClear {
        room: &'static RuleRoom,
    },
    Cleared {
        room: &'static RuleRoom,
        player: RulePlayer,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestBreakVerdict {
    NotARuleBreak,
    RunsOnce {
        room: &'static RuleRoom,
        act: RuleAct,
    },
    NotArmed {
        room: &'static RuleRoom,
        state: Option<RuleState>,
    },
}

/// This game's view of the rule room it is in: armed by the room's break handlers registering,
/// broken once by the first break whoever made it, cleared by the room's clear.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuleRoomWatch {
    current: Option<(&'static RuleRoom, RuleState)>,
}

impl RulePop {
    pub fn drops(self) -> bool {
        matches!(
            self,
            Self::GuestAsksTheHost { .. }
                | Self::DropAlreadyBroken { .. }
                | Self::DropGuestTimeUp { .. }
                | Self::DropGuestRestart { .. }
                | Self::DropGuestClear { .. }
        )
    }

    pub fn skips_the_story_relay(self) -> bool {
        matches!(self, Self::GuestStartRunsHereOnly { .. })
    }
}

impl RuleRoomWatch {
    pub const fn new() -> Self {
        Self { current: None }
    }

    pub fn state_of(&self, room: &RuleRoom) -> Option<RuleState> {
        self.current
            .filter(|(current, _)| *current == room)
            .map(|(_, state)| state)
    }

    pub fn armed_room(&self, pack: &str) -> Option<&'static RuleRoom> {
        self.current
            .filter(|(room, state)| room.pack == pack && *state == RuleState::Armed)
            .map(|(room, _)| room)
    }

    /// A break handler of a rule room registered in `pack`: returns the room when this arms it.
    pub fn registered(&mut self, pack: &str, handler: &str) -> Option<&'static RuleRoom> {
        let room = RULE_ROOMS
            .iter()
            .find(|room| room.pack == pack && room.act_of(handler).is_some())?;
        if self.state_of(room) == Some(RuleState::Armed) {
            return None;
        }
        self.current = Some((room, RuleState::Armed));
        Some(room)
    }

    pub fn pop(&mut self, facts: &RulePopFacts) -> RulePop {
        let Some((room, call)) = rule_room_call(facts.pack, facts.function) else {
            return RulePop::NotARuleRoomCall;
        };
        let shared_guest = facts.shared && facts.player == RulePlayer::Guest;
        let guest_own = shared_guest && facts.own_fire;
        match call {
            RuleRoomCall::Start if guest_own => RulePop::GuestStartRunsHereOnly { room },
            RuleRoomCall::Start if shared_guest && facts.queued_by_the_mod => {
                RulePop::GuestTakenIn { room }
            }
            // Every break handler's timer fires the restart in both games; only the host's resets.
            RuleRoomCall::Restart if shared_guest && !facts.queued_by_the_mod => {
                RulePop::DropGuestRestart { room }
            }
            RuleRoomCall::Start | RuleRoomCall::Restart => RulePop::PassesOn,
            RuleRoomCall::Clear if guest_own => RulePop::DropGuestClear { room },
            RuleRoomCall::Clear => {
                self.current = Some((room, RuleState::Cleared));
                RulePop::Cleared {
                    room,
                    player: facts.player,
                }
            }
            RuleRoomCall::Break(act) => self.break_popped(facts, room, act),
        }
    }

    pub fn accept_guest_break(&mut self, pack: &str, handler: &str) -> GuestBreakVerdict {
        let Some((room, RuleRoomCall::Break(act))) = rule_room_call(pack, handler) else {
            return GuestBreakVerdict::NotARuleBreak;
        };
        if !act.is_a_players_act() {
            return GuestBreakVerdict::NotARuleBreak;
        }
        let state = self.state_of(room);
        if state != Some(RuleState::Armed) {
            return GuestBreakVerdict::NotArmed { room, state };
        }
        self.current = Some((room, RuleState::AskedOfTheHost));
        GuestBreakVerdict::RunsOnce { room, act }
    }

    /// The host's own post of `message` stays out of the mirror while it is a break message of
    /// the room armed here: the host's break reaches the guest as the host's run, never twice.
    pub fn keeps_message_here(&self, pack: &str, message: u32) -> Option<&'static RuleRoom> {
        self.armed_room(pack)
            .filter(|room| room.has_break_message(message))
    }

    fn break_popped(
        &mut self,
        facts: &RulePopFacts,
        room: &'static RuleRoom,
        act: RuleAct,
    ) -> RulePop {
        let state = self.state_of(room);
        if !facts.shared {
            self.current = Some((room, RuleState::Broken));
            return RulePop::BrokenAlone { room, act };
        }
        match (facts.player, facts.own_fire) {
            (RulePlayer::Guest, true) if !act.is_a_players_act() => {
                RulePop::DropGuestTimeUp { room }
            }
            (RulePlayer::Guest, true) => match state {
                Some(RuleState::AskedOfTheHost | RuleState::Broken | RuleState::Cleared) => {
                    RulePop::DropAlreadyBroken {
                        room,
                        act,
                        player: RulePlayer::Guest,
                    }
                }
                Some(RuleState::Armed) | None => {
                    self.current = Some((room, RuleState::AskedOfTheHost));
                    RulePop::GuestAsksTheHost { room, act }
                }
            },
            (RulePlayer::Guest, false) => {
                self.current = Some((room, RuleState::Broken));
                RulePop::HostsBreakRunsHere { room, act }
            }
            (RulePlayer::Host, true) => match state {
                Some(RuleState::AskedOfTheHost | RuleState::Broken) => RulePop::DropAlreadyBroken {
                    room,
                    act,
                    player: RulePlayer::Host,
                },
                Some(RuleState::Armed | RuleState::Cleared) | None => {
                    self.current = Some((room, RuleState::Broken));
                    RulePop::HostBreaks { room, act }
                }
            },
            (RulePlayer::Host, false) => {
                self.current = Some((room, RuleState::Broken));
                RulePop::GuestsBreakRunsHere { room, act }
            }
        }
    }
}
