use replicant_proto::PartPickup;

use crate::part_pickups::MAX_PENDING_PART_PICKUPS;
use crate::part_pickups::errands::PartPickupErrand;

/// The partner's pickups not yet applied here: they wait until this copy's actor counts its
/// parts, so the actor's own setup cannot show a part the partner already took.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PendingPartPickups {
    pickups: Vec<PartPickup>,
}

impl PendingPartPickups {
    pub const fn new() -> Self {
        Self {
            pickups: Vec::new(),
        }
    }

    pub fn queue(&mut self, pickup: PartPickup) -> bool {
        if self.pickups.contains(&pickup) || self.pickups.len() >= MAX_PENDING_PART_PICKUPS {
            return false;
        }
        self.pickups.push(pickup);
        true
    }

    pub fn take_for_counting_step(
        &mut self,
        errand: &PartPickupErrand,
        stage: i32,
        phase: Option<u16>,
    ) -> Vec<u16> {
        if stage != errand.counting_stage {
            return Vec::new();
        }
        let Some(phase) = phase else {
            return Vec::new();
        };
        let mut taken = Vec::new();
        self.pickups.retain(|pickup| {
            let applies =
                pickup.actor_handler_rva == errand.actor_handler_rva && pickup.phase == phase;
            if applies {
                taken.push(pickup.part_group);
            }
            !applies
        });
        taken
    }

    pub fn drop_other_phases(&mut self, phase: Option<u16>) -> usize {
        let before = self.pickups.len();
        self.pickups.retain(|pickup| Some(pickup.phase) == phase);
        before - self.pickups.len()
    }

    pub fn clear(&mut self) {
        self.pickups.clear();
    }

    pub fn len(&self) -> usize {
        self.pickups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pickups.is_empty()
    }
}
