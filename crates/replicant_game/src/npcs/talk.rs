use std::collections::BTreeMap;

use replicant_proto::{NpcId, NpcState, NpcTalk};

use crate::npcs::{NPC_TALK_HOLD_EXPIRY_MS, NPC_TALK_RESEND_MS};

/// NPCs the partner is talking to, with the partner's latest pose for each. A hold ends when the
/// partner says so, on a phase change, or when no update came for the expiry time.
#[derive(Debug, Default)]
pub struct PartnerTalks {
    phase: Option<u16>,
    holds: BTreeMap<NpcId, (NpcState, u64)>,
}

/// This machine's own conversations: what to tell the partner, and when.
#[derive(Debug, Default)]
pub struct OwnTalks {
    phase: Option<u16>,
    last_sent: BTreeMap<NpcId, u64>,
}

impl PartnerTalks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn receive(&mut self, at_ms: u64, talk: &NpcTalk) {
        if self.phase != Some(talk.phase) {
            self.holds.clear();
            self.phase = Some(talk.phase);
        }
        if talk.talking {
            self.holds.insert(talk.npc.id, (talk.npc, at_ms));
        } else {
            self.holds.remove(&talk.npc.id);
        }
    }

    /// The partner's pose for each NPC it is talking to in `phase`, dropping expired holds.
    pub fn active(&mut self, now_ms: u64, phase: u16) -> BTreeMap<NpcId, NpcState> {
        if self.phase != Some(phase) {
            self.holds.clear();
            return BTreeMap::new();
        }
        self.holds
            .retain(|_, (_, at)| now_ms.saturating_sub(*at) <= NPC_TALK_HOLD_EXPIRY_MS);
        self.holds
            .iter()
            .map(|(id, (state, _))| (*id, *state))
            .collect()
    }
}

impl OwnTalks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The messages to send now, given the NPCs this machine's player is talking to: each talk
    /// is repeated every resend time while it lasts, and one ending message follows when it ends.
    pub fn update(&mut self, now_ms: u64, phase: u16, talking: &[NpcState]) -> Vec<NpcTalk> {
        let mut out = Vec::new();
        if self.phase != Some(phase) {
            self.last_sent.clear();
            self.phase = Some(phase);
        }
        let ended: Vec<NpcId> = self
            .last_sent
            .keys()
            .filter(|id| !talking.iter().any(|npc| npc.id == **id))
            .copied()
            .collect();
        for id in ended {
            self.last_sent.remove(&id);
            out.push(NpcTalk {
                phase,
                npc: NpcState {
                    id,
                    position: [0.0; 3],
                    yaw_radians: 0.0,
                    motion: None,
                    visible: true,
                },
                talking: false,
            });
        }
        for npc in talking {
            let due = self
                .last_sent
                .get(&npc.id)
                .is_none_or(|sent| now_ms.saturating_sub(*sent) >= NPC_TALK_RESEND_MS);
            if due {
                self.last_sent.insert(npc.id, now_ms);
                out.push(NpcTalk {
                    phase,
                    npc: *npc,
                    talking: true,
                });
            }
        }
        out
    }
}
