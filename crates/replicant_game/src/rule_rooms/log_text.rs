use crate::rule_rooms::{
    ReturnSpot, RoomResetVerdict, RuleAct, RulePlayer, RulePop, RuleRoom, RuleState,
};

impl RulePlayer {
    pub fn label(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Guest => "guest",
        }
    }
}

impl RulePop {
    pub fn log_line(&self, function: &str) -> Option<String> {
        let line = match *self {
            Self::NotARuleRoomCall | Self::PassesOn | Self::GuestTakenIn { .. } => return None,
            Self::GuestStartRunsHereOnly { room } => format!(
                "rule room: {} starts in your game from your own entry, it runs here and is not asked of the host",
                room_name(room)
            ),
            Self::BrokenAlone { room, act } => format!(
                "rule room: {} broken here ({}, {function}), the partner is not in this phase, it resets only here",
                room_name(room),
                act.label()
            ),
            Self::ArmRunsHere { room } => format!(
                "rule room: {}: {function} runs here to register the breaks again, room not reset",
                room_name(room)
            ),
            Self::BreakerSentBack { room, act, player } => {
                if act.is_a_players_act() {
                    format!(
                        "rule room: {}: the {} broke the rule ({}, {function}), only the {} goes back to the start, room not reset",
                        room_name(room),
                        player.label(),
                        act.label(),
                        player.label()
                    )
                } else {
                    format!(
                        "rule room: {}: the time ran out in the {}'s game ({function}), the {} goes back to the start on its own clock, room not reset",
                        room_name(room),
                        player.label(),
                        player.label()
                    )
                }
            }
            Self::DropAlreadyBroken { room, act, player } => format!(
                "rule room: {}: the {}'s {function} ({}) not run, the {} already broke the rule and is on the way back",
                room_name(room),
                player.label(),
                act.label(),
                player.label()
            ),
            Self::DropPartnersBreak { room, act, player } => format!(
                "rule room: {}: the partner's {function} ({}) not run on the {}'s game, a break sends back only the one who broke it",
                room_name(room),
                act.label(),
                player.label()
            ),
            Self::HostBreaks { room, act } => format!(
                "rule room: {}: the host broke the rule ({}, {function}), the room resets as one: the host's restart runs on both, objects are placed again",
                room_name(room),
                act.label()
            ),
            Self::HostsBreakRunsHere { room, act } => format!(
                "rule room: {}: the host's break ({}, {function}) runs here",
                room_name(room),
                act.label()
            ),
            Self::GuestsBreakRunsHere { room, act } => format!(
                "rule room: {}: the guest's break ({}, {function}) runs here once, the reset reaches both",
                room_name(room),
                act.label()
            ),
            Self::RoomRestarts { room } => format!(
                "rule room: {}: {function} runs, the room's full reset, the clock is armed again after it",
                room_name(room)
            ),
            Self::DropGuestRestart { room } => format!(
                "rule room: {}: your game's {function} not run, the host's restart resets both together",
                room_name(room)
            ),
            Self::GuestAsksTheHost { room, act } => format!(
                "rule room: {}: the guest broke the rule ({}, {function}), not run here, asked of the host: the host resets the room for both",
                room_name(room),
                act.label()
            ),
            Self::DropGuestTimeUp { room } => format!(
                "rule room: {}: your own timer ({function}) ran out, not run here, the host's clock decides",
                room_name(room)
            ),
            Self::DropRestart { room } => format!(
                "rule room: {}: {function} not run, a break never resets the room",
                room_name(room)
            ),
            Self::DropGuestClear { room } => format!(
                "rule room: {}: your game's {function} not run, the host clears the room",
                room_name(room)
            ),
            Self::Cleared { room, player } => format!(
                "rule room: {} cleared ({function} on the {} side, flag {})",
                room_name(room),
                player.label(),
                room.clear_flag
            ),
        };
        Some(line)
    }
}

impl RuleState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Armed => "armed",
            Self::Broken => "already broken",
            Self::Cleared => "cleared",
        }
    }
}

pub fn armed_line(room: &RuleRoom, player: RulePlayer, handler: &str) -> String {
    let breaks = room
        .breaks
        .iter()
        .map(|rule_break| format!("{} {}", rule_break.act.label(), rule_break.handler))
        .collect::<Vec<_>>()
        .join(", ");
    let inhibits = if room.inhibits.is_empty() {
        "none".to_owned()
    } else {
        room.inhibits
            .iter()
            .map(|inhibit| inhibit.label())
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "rule room: {} armed on the {} side by {handler} (breaks: {breaks}; inhibits: {inhibits})",
        room_name(room),
        player.label()
    )
}

pub fn returned_line(room: &RuleRoom, player: RulePlayer, spot: &ReturnSpot) -> String {
    format!(
        "rule room: {}: the {} was sent back to ({:.1}, {:.1}, {:.1}), room not reset: the partner and every object stay where they are",
        room_name(room),
        player.label(),
        spot.position[0],
        spot.position[1],
        spot.position[2]
    )
}

pub fn returned_without_a_spot_line(room: &RuleRoom, act: RuleAct) -> String {
    format!(
        "rule room: {}: no start spot is known for the {} break, nothing moved",
        room_name(room),
        act.label()
    )
}

pub fn kept_message_line(room: &RuleRoom, message: u32) -> String {
    format!(
        "boss message: host raised {message}, kept here: it breaks the rule of {}, it only sends the host back",
        room_name(room)
    )
}

pub(crate) fn room_name(room: &RuleRoom) -> String {
    format!("room {} of {}", room.room, room.pack)
}

pub fn guest_break_line(verdict: &RoomResetVerdict, handler: &str) -> String {
    match *verdict {
        RoomResetVerdict::NotAResetBreak => {
            format!("rule room: the guest's {handler} breaks no room that resets as one, not run")
        }
        RoomResetVerdict::Resets { room, act } => format!(
            "rule room: {}: the guest broke the rule ({}, {handler}), the host resets the room for both players",
            room_name(room),
            act.label()
        ),
        RoomResetVerdict::NotArmed { room, state } => format!(
            "rule room: {}: the guest's {handler} not run, the rule here is {}",
            room_name(room),
            state.map_or("not armed", RuleState::label)
        ),
    }
}
