use std::collections::BTreeMap;

use crate::co_op_log::{CoOpLines, PlayerSeen, SessionSeen};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OtherSeen<'a> {
    pub slot: u8,
    pub name: &'a str,
    pub state: Option<PlayerSeen>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ManyCoOpLines {
    local_up: Option<bool>,
    by_slot: BTreeMap<u8, CoOpLines>,
}

impl ManyCoOpLines {
    pub const fn new() -> Self {
        Self {
            local_up: None,
            by_slot: BTreeMap::new(),
        }
    }

    /// One player's lines per other player, in slot order; a player who is no longer listed
    /// leaves. The local defeat is told once, not once per player.
    pub fn note(
        &mut self,
        connected: bool,
        local: Option<PlayerSeen>,
        others: &[OtherSeen<'_>],
        now_ms: u64,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(up) = local.map(|local| local.health > 0)
            && self.local_up.replace(up) == Some(true)
            && !up
        {
            lines.push("You were defeated".to_owned());
        }
        let gone: Vec<u8> = self
            .by_slot
            .keys()
            .copied()
            .filter(|slot| !others.iter().any(|other| other.slot == *slot))
            .collect();
        for slot in gone {
            if let Some(mut lines_of) = self.by_slot.remove(&slot) {
                let seen = SessionSeen {
                    connected: false,
                    partner_name: "",
                    local,
                    partner: None,
                };
                lines.extend(lines_of.note_partner(&seen, now_ms));
            }
        }
        let mut sorted: Vec<&OtherSeen<'_>> = others.iter().collect();
        sorted.sort_by_key(|other| other.slot);
        for other in sorted {
            let seen = SessionSeen {
                connected,
                partner_name: other.name,
                local,
                partner: other.state,
            };
            lines.extend(
                self.by_slot
                    .entry(other.slot)
                    .or_default()
                    .note_partner(&seen, now_ms),
            );
        }
        lines
    }
}
