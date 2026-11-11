use crate::co_op_log::PLACE_CHANGE_HOLDS_FOR_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSeen {
    pub phase_id: i32,
    pub health: i32,
}

/// What one game tick knows of the session; a player is `None` until a state of theirs is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionSeen<'a> {
    pub connected: bool,
    pub partner_name: &'a str,
    pub local: Option<PlayerSeen>,
    pub partner: Option<PlayerSeen>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlaceReading {
    together: bool,
    since_ms: u64,
}

/// Turns what the link shows, tick by tick, into the co-op lines a player reads.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoOpLines {
    joined_as: Option<String>,
    told_together: Option<bool>,
    place: Option<PlaceReading>,
    partner_up: Option<bool>,
    local_up: Option<bool>,
}

impl CoOpLines {
    pub const fn new() -> Self {
        Self {
            joined_as: None,
            told_together: None,
            place: None,
            partner_up: None,
            local_up: None,
        }
    }

    pub fn note(&mut self, seen: &SessionSeen<'_>, now_ms: u64) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(up) = seen.local.map(|local| local.health > 0)
            && self.local_up.replace(up) == Some(true)
            && !up
        {
            lines.push("You were defeated".to_owned());
        }
        if !seen.connected {
            if let Some(name) = self.joined_as.take() {
                lines.push(format!("{name} left"));
            }
            self.told_together = None;
            self.place = None;
            self.partner_up = None;
            return lines;
        }
        let Some(partner) = seen.partner else {
            return lines;
        };
        let name = seen.partner_name;
        if self.joined_as.is_none() {
            self.joined_as = Some(name.to_owned());
            lines.push(format!("{name} joined"));
        }
        let up = partner.health > 0;
        match (self.partner_up.replace(up), up) {
            (Some(true), false) => lines.push(format!("{name} was defeated")),
            (Some(false), true) => lines.push(format!("{name} is back up")),
            _ => {}
        }
        if let Some(local) = seen.local {
            lines.extend(self.place_line(name, local.phase_id == partner.phase_id, now_ms));
        }
        lines
    }

    fn place_line(&mut self, name: &str, together: bool, now_ms: u64) -> Option<String> {
        let reading = match self.place {
            Some(reading) if reading.together == together => reading,
            _ => PlaceReading {
                together,
                since_ms: now_ms,
            },
        };
        self.place = Some(reading);
        // A load passes through other places for a moment: only a reading that holds is told.
        if now_ms.saturating_sub(reading.since_ms) < PLACE_CHANGE_HOLDS_FOR_MS
            || self.told_together == Some(together)
        {
            return None;
        }
        let told_before = self.told_together.replace(together);
        match (told_before, together) {
            (Some(true), false) => Some(format!("{name} went to another place")),
            (None, false) => Some(format!("{name} is in another place")),
            (Some(false), true) => Some(format!("{name} is here again")),
            _ => None,
        }
    }
}
