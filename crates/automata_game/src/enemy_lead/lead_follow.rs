use std::collections::HashMap;

use crate::enemies::EnemyId;
use crate::enemy_lead::{
    ENEMY_IS_HELD_ONCE_IT_HAS_LIVED_HERE_MS, EnemyLead, EnemyPose,
    HOLD_LIFTS_AFTER_THE_HOSTS_SILENCE_MS, HOST_STARTS_LEFT_TO_THE_DEFEAT_PATH, MOST_ENEMIES_LED,
};

/// The guest's side of the host's enemy stream: the last pose heard per enemy, which of its
/// state starts this copy has played, and since when this copy's body of it lives.
#[derive(Debug, Clone, Default)]
pub struct LeadFollower {
    heard: HashMap<EnemyId, HeardPose>,
    bodies: HashMap<EnemyId, LocalBody>,
}

#[derive(Debug, Clone, Copy)]
struct HeardPose {
    pose: EnemyPose,
    heard_at_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct LocalBody {
    handle: u32,
    since_ms: u64,
    starts_played: Option<u32>,
}

impl LeadFollower {
    pub fn hear(&mut self, now_ms: u64, lead: &EnemyLead) {
        for pose in &lead.poses {
            if self.heard.len() >= MOST_ENEMIES_LED && !self.heard.contains_key(&pose.id) {
                continue;
            }
            self.heard.insert(
                pose.id,
                HeardPose {
                    pose: *pose,
                    heard_at_ms: now_ms,
                },
            );
        }
    }

    /// The host's last pose of an enemy while its stream for that enemy is not older than the
    /// hold allows; none lets the enemy run by itself.
    pub fn fresh_pose(&self, id: EnemyId, now_ms: u64) -> Option<EnemyPose> {
        self.heard
            .get(&id)
            .filter(|heard| {
                now_ms.saturating_sub(heard.heard_at_ms) < HOLD_LIFTS_AFTER_THE_HOSTS_SILENCE_MS
            })
            .map(|heard| heard.pose)
    }

    pub fn heard_at_ms(&self, id: EnemyId) -> Option<u64> {
        self.heard.get(&id).map(|heard| heard.heard_at_ms)
    }

    /// Notes the body this copy holds for an enemy now; a new handle starts its time here again.
    pub fn note_body(&mut self, id: EnemyId, handle: u32, now_ms: u64) {
        let body = self.bodies.entry(id).or_insert(LocalBody {
            handle,
            since_ms: now_ms,
            starts_played: None,
        });
        if body.handle != handle {
            *body = LocalBody {
                handle,
                since_ms: now_ms,
                starts_played: None,
            };
        }
    }

    /// Whether this copy's own choices for the enemy are held: the host's stream for it is
    /// fresh and this copy's body has lived long enough to have finished its own birth.
    pub fn holds(&self, id: EnemyId, now_ms: u64) -> bool {
        self.fresh_pose(id, now_ms).is_some()
            && self.bodies.get(&id).is_some_and(|body| {
                now_ms.saturating_sub(body.since_ms) >= ENEMY_IS_HELD_ONCE_IT_HAS_LIVED_HERE_MS
            })
    }

    /// The host's state to start on this copy's body, once per state start the host counted. A
    /// defeat state is left to the defeat the host tells, which starts it here by itself.
    pub fn start_to_play(&mut self, id: EnemyId, now_ms: u64) -> Option<[i32; 4]> {
        if !self.holds(id, now_ms) {
            return None;
        }
        let pose = self.fresh_pose(id, now_ms)?;
        let body = self.bodies.get_mut(&id)?;
        if body.starts_played == Some(pose.starts) {
            return None;
        }
        body.starts_played = Some(pose.starts);
        (!HOST_STARTS_LEFT_TO_THE_DEFEAT_PATH.contains(&pose.state[0])).then_some(pose.state)
    }

    /// Drops the bodies of enemies this copy no longer holds, and poses of enemies it holds no
    /// body for that the host has not told for as long as the hold lasts.
    pub fn keep_only(&mut self, ids: &[EnemyId], now_ms: u64) {
        self.bodies.retain(|id, _| ids.contains(id));
        self.heard.retain(|id, heard| {
            ids.contains(id)
                || now_ms.saturating_sub(heard.heard_at_ms) < HOLD_LIFTS_AFTER_THE_HOSTS_SILENCE_MS
        });
    }

    pub fn forget(&mut self) {
        self.heard.clear();
        self.bodies.clear();
    }
}
