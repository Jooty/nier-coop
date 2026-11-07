use serde::{Deserialize, Serialize};

use crate::wire::{ENEMY_GROUP_COUNT, MAX_POSITION_ABS, WireError};

pub const MAX_DESYNC_GROUPS: usize = 32;
pub const PARTNER_POSITION_DESYNC_DISTANCE: f32 = 15.0;
pub const MAX_DESYNC_GROUP_LINES: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupRoster {
    pub group: u16,
    pub count: u16,
    pub ids_hash: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DesyncDigest {
    pub phase: u16,
    pub position: [f32; 3],
    pub story: Option<u64>,
    pub groups: Vec<GroupRoster>,
    pub groups_not_listed: u16,
}

impl DesyncDigest {
    pub fn validate(&self) -> Result<(), WireError> {
        if self
            .position
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS)
        {
            return Err(WireError::OutOfRange { field: "position" });
        }
        let sorted = self.groups.windows(2).all(|pair| pair[0].group < pair[1].group);
        if self.groups.len() > MAX_DESYNC_GROUPS
            || !sorted
            || self.groups.iter().any(|roster| roster.group >= ENEMY_GROUP_COUNT)
        {
            return Err(WireError::OutOfRange { field: "groups" });
        }
        Ok(())
    }

    pub fn differences(
        &self,
        partner: &DesyncDigest,
        partner_position_seen: Option<[f32; 3]>,
    ) -> Vec<String> {
        if self.phase != partner.phase {
            return vec![format!(
                "phase (yours {}, partner {})",
                self.phase, partner.phase
            )];
        }
        let mut found = Vec::new();
        if let (Some(mine), Some(theirs)) = (self.story, partner.story)
            && mine != theirs
        {
            found.push(format!(
                "story digest (yours {mine:016x}, partner {theirs:016x})"
            ));
        }
        found.extend(self.group_differences(partner));
        if let Some(seen) = partner_position_seen
            && distance(seen, partner.position) > PARTNER_POSITION_DESYNC_DISTANCE
        {
            found.push(format!(
                "partner position (yours sees {}, partner reports {})",
                position_text(seen),
                position_text(partner.position)
            ));
        }
        found
    }

    fn group_differences(&self, partner: &DesyncDigest) -> Vec<String> {
        let mut groups: Vec<u16> = self
            .groups
            .iter()
            .chain(&partner.groups)
            .map(|roster| roster.group)
            .collect();
        groups.sort_unstable();
        groups.dedup();
        let mut lines: Vec<String> = groups
            .into_iter()
            .filter_map(|group| {
                let mine = self.roster_of(group);
                let theirs = partner.roster_of(group);
                (mine != theirs).then(|| {
                    format!(
                        "enemy group {group} (yours {}, partner {})",
                        roster_text(mine),
                        roster_text(theirs)
                    )
                })
            })
            .collect();
        if lines.len() > MAX_DESYNC_GROUP_LINES {
            let more = lines.len() - MAX_DESYNC_GROUP_LINES;
            lines.truncate(MAX_DESYNC_GROUP_LINES);
            lines.push(format!("enemy groups, {more} more differ (yours, partner)"));
        }
        if self.groups_not_listed != partner.groups_not_listed {
            lines.push(format!(
                "enemy groups beyond the listed ones (yours {}, partner {})",
                self.groups_not_listed, partner.groups_not_listed
            ));
        }
        lines
    }

    fn roster_of(&self, group: u16) -> Option<(u16, u32)> {
        self.groups
            .iter()
            .find(|roster| roster.group == group)
            .map(|roster| (roster.count, roster.ids_hash))
    }
}

fn roster_text(roster: Option<(u16, u32)>) -> String {
    match roster {
        Some((count, hash)) => format!("{count} live, ids {hash:08x}"),
        None => "none".to_owned(),
    }
}

fn position_text(position: [f32; 3]) -> String {
    format!("{:.0},{:.0},{:.0}", position[0], position[1], position[2])
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}
