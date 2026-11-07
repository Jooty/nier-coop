use rc_proto::EnemyId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BossPick {
    pub id: EnemyId,
    pub max_hp: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossFightChange {
    On(BossPick),
    Off,
}

#[derive(Debug, Default)]
pub struct BossFight {
    raised_in_phase: Option<u16>,
    boss: Option<BossPick>,
}

impl BossFight {
    pub const fn new() -> Self {
        Self {
            raised_in_phase: None,
            boss: None,
        }
    }

    pub fn raise(&mut self, phase: u16) {
        if self.raised_in_phase != Some(phase) {
            self.raised_in_phase = Some(phase);
            self.boss = None;
        }
    }

    pub fn is_active(&self) -> bool {
        self.raised_in_phase.is_some()
    }

    pub fn is_boss(&self, id: EnemyId) -> bool {
        self.boss.is_some_and(|boss| boss.id == id)
    }

    pub fn boss(&self) -> Option<BossPick> {
        self.boss
    }

    pub fn observe(
        &mut self,
        phase: u16,
        enemies: &[(EnemyId, i32, i32)],
    ) -> Option<BossFightChange> {
        let raised_in = self.raised_in_phase?;
        if raised_in != phase {
            self.raised_in_phase = None;
            self.boss = None;
            return Some(BossFightChange::Off);
        }
        let strongest_live = enemies
            .iter()
            .filter(|(_, hp, _)| *hp > 0)
            .max_by_key(|(_, _, max_hp)| *max_hp)
            .map(|(id, _, max_hp)| BossPick {
                id: *id,
                max_hp: *max_hp,
            })?;
        let keep_current = self.boss.is_some_and(|current| {
            current.max_hp >= strongest_live.max_hp
                && enemies.iter().any(|(id, _, _)| *id == current.id)
        });
        if keep_current {
            return None;
        }
        self.boss = Some(strongest_live);
        Some(BossFightChange::On(strongest_live))
    }
}
