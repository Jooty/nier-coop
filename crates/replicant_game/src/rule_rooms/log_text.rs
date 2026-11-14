use crate::rule_rooms::{ReturnSpot, RuleAct, RulePlayer, RulePop, RuleRoom};

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
