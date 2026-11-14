use crate::rule_rooms::{GuestBreakVerdict, RulePlayer, RulePop, RuleRoom, RuleState};

impl RulePlayer {
    pub fn label(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Guest => "guest",
        }
    }
}

impl RuleState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Armed => "armed",
            Self::AskedOfTheHost => "already asked",
            Self::Broken => "already broken",
            Self::Cleared => "cleared",
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
            Self::HostBreaks { room, act } => format!(
                "rule room: {} broken by the host ({}, {function}), it runs here once and the reset reaches the guest",
                room_name(room),
                act.label()
            ),
            Self::GuestAsksTheHost { room, act } => format!(
                "rule room: {} broken by the guest ({}, {function}), not run here, asked of the host",
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
            Self::DropAlreadyBroken { room, act, player } => format!(
                "rule room: {}: the {}'s {function} ({}) not run, the rule already broke once",
                room_name(room),
                player.label(),
                act.label()
            ),
            Self::DropGuestTimeUp { room } => format!(
                "rule room: {}: your game's {function} not run, the host's clock times the room",
                room_name(room)
            ),
            Self::DropGuestRestart { room } => format!(
                "rule room: {}: your game's {function} not run, the host's restart resets both together",
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

pub fn guest_break_line(verdict: &GuestBreakVerdict, handler: &str) -> String {
    match *verdict {
        GuestBreakVerdict::NotARuleBreak => {
            format!("rule room: the guest's {handler} is no player's rule break here, not run")
        }
        GuestBreakVerdict::RunsOnce { room, act } => format!(
            "rule room: {}: the guest broke the rule ({}, {handler}), it runs here once",
            room_name(room),
            act.label()
        ),
        GuestBreakVerdict::NotArmed { room, state } => format!(
            "rule room: {}: the guest's {handler} not run, the rule here is {}",
            room_name(room),
            state.map_or("not armed", RuleState::label)
        ),
    }
}

pub fn kept_message_line(room: &RuleRoom, message: u32) -> String {
    format!(
        "boss message: host raised {message}, kept here: it breaks the rule of {}, the host's run reaches the guest",
        room_name(room)
    )
}

pub(crate) fn room_name(room: &RuleRoom) -> String {
    format!("room {} of {}", room.room, room.pack)
}
