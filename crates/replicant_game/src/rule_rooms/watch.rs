use crate::rule_rooms::{RULE_ROOMS, RuleAct, RuleRoom, RuleRoomCall, rule_room_call};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulePlayer {
    Host,
    Guest,
}

/// Where this game's current rule room stands: armed by the room's break handlers registering,
/// broken by the first break of this game's own player, cleared by the room's clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleState {
    Armed,
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
    ArmRunsHere {
        room: &'static RuleRoom,
    },
    BrokenAlone {
        room: &'static RuleRoom,
        act: RuleAct,
    },
    BreakerSentBack {
        room: &'static RuleRoom,
        act: RuleAct,
        player: RulePlayer,
    },
    DropAlreadyBroken {
        room: &'static RuleRoom,
        act: RuleAct,
        player: RulePlayer,
    },
    DropPartnersBreak {
        room: &'static RuleRoom,
        act: RuleAct,
        player: RulePlayer,
    },
    DropRestart {
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

/// This game's view of the rule room it is in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuleRoomWatch {
    current: Option<(&'static RuleRoom, RuleState)>,
}

impl RulePop {
    pub fn drops(self) -> bool {
        matches!(
            self,
            Self::BreakerSentBack { .. }
                | Self::DropAlreadyBroken { .. }
                | Self::DropPartnersBreak { .. }
                | Self::DropRestart { .. }
                | Self::DropGuestClear { .. }
        )
    }

    pub fn skips_the_story_relay(self) -> bool {
        matches!(self, Self::GuestStartRunsHereOnly { .. })
    }

    pub fn runs_here_outside_the_streams(self) -> bool {
        matches!(self, Self::ArmRunsHere { .. })
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
            RuleRoomCall::Start => RulePop::PassesOn,
            RuleRoomCall::Arm if facts.queued_by_the_mod => RulePop::ArmRunsHere { room },
            RuleRoomCall::Arm => RulePop::PassesOn,
            RuleRoomCall::Restart if facts.shared => RulePop::DropRestart { room },
            RuleRoomCall::Restart => RulePop::PassesOn,
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

    /// The host's own post of `message` stays out of the mirror while it is a break message of the
    /// room this game is in: a break sends back only the player whose game raised it.
    pub fn keeps_message_here(&self, pack: &str, message: u32) -> Option<&'static RuleRoom> {
        self.current
            .filter(|(room, state)| room.pack == pack && *state != RuleState::Cleared)
            .map(|(room, _)| room)
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
        let player = facts.player;
        if facts.queued_by_the_mod {
            return RulePop::DropPartnersBreak { room, act, player };
        }
        match state {
            Some(RuleState::Broken | RuleState::Cleared) => {
                RulePop::DropAlreadyBroken { room, act, player }
            }
            Some(RuleState::Armed) | None => {
                self.current = Some((room, RuleState::Broken));
                RulePop::BreakerSentBack { room, act, player }
            }
        }
    }
}
