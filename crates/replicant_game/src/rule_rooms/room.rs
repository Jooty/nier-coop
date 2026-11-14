use crate::rule_rooms::{
    PLAYER_ATTACKED_MESSAGE, PLAYER_CAST_MAGIC_MESSAGES, PLAYER_DASHED_MESSAGE,
    PLAYER_GUARDED_MESSAGE, PLAYER_HIT_BY_THE_ROOM_MESSAGE, PLAYER_JUMPED_MESSAGE,
};

/// What a player does that breaks a room's rule; `TimeUp` is the room's own clock, no player's act.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleAct {
    Jump,
    HitByTheRoom,
    Move,
    Run,
    Dash,
    Magic,
    Guard,
    Attack,
    TimeUp,
}

/// An auto-battle inhibit a room's set-up function turns on in the game that runs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomInhibit {
    Weapon,
    Magic,
    Jump,
    LockOn,
    Avoid,
    Guard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleBreak {
    pub handler: &'static str,
    pub act: RuleAct,
}

/// One room whose rule a player can break: `starts` arm it on entry, `arms` register its breaks
/// again, `clear` sets `clear_flag` and opens the way on. A break in a shared session sends only
/// the breaker back and runs `arms`, never `restart` (which would re-place the room's objects);
/// a room with `resets_the_room` sends both players back together on the host's decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleRoom {
    pub pack: &'static str,
    pub room: u8,
    pub starts: &'static [&'static str],
    pub restart: &'static str,
    pub arms: &'static [&'static str],
    pub removed_before_arming: &'static [i32],
    pub resets_the_room: bool,
    pub clear: &'static str,
    pub clear_flag: u32,
    pub breaks: &'static [RuleBreak],
    pub inhibits: &'static [RoomInhibit],
}

impl RuleAct {
    /// The messages whose callback is this act's break handler; empty for a move or timer callback.
    pub fn messages(self) -> &'static [u32] {
        match self {
            Self::Jump => &[PLAYER_JUMPED_MESSAGE],
            Self::HitByTheRoom => &[PLAYER_HIT_BY_THE_ROOM_MESSAGE],
            Self::Dash => &[PLAYER_DASHED_MESSAGE],
            Self::Magic => &PLAYER_CAST_MAGIC_MESSAGES,
            Self::Guard => &[PLAYER_GUARDED_MESSAGE],
            Self::Attack => &[PLAYER_ATTACKED_MESSAGE],
            Self::Move | Self::Run | Self::TimeUp => &[],
        }
    }

    pub fn is_a_players_act(self) -> bool {
        self != Self::TimeUp
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Jump => "jump",
            Self::HitByTheRoom => "hit",
            Self::Move => "move",
            Self::Run => "run",
            Self::Dash => "dash",
            Self::Magic => "magic",
            Self::Guard => "guard",
            Self::Attack => "attack",
            Self::TimeUp => "time up",
        }
    }
}

impl RoomInhibit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Weapon => "weapon",
            Self::Magic => "magic",
            Self::Jump => "jump",
            Self::LockOn => "lock-on",
            Self::Avoid => "avoid",
            Self::Guard => "guard",
        }
    }
}

impl RuleRoom {
    pub fn act_of(&self, handler: &str) -> Option<RuleAct> {
        self.breaks
            .iter()
            .find(|rule_break| rule_break.handler == handler)
            .map(|rule_break| rule_break.act)
    }

    pub fn has_break_message(&self, message: u32) -> bool {
        self.breaks
            .iter()
            .any(|rule_break| rule_break.act.messages().contains(&message))
    }
}
