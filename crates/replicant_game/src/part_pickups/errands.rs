/// A native errand where a player picks parts off an actor's model: the actor's event handler
/// hides a part on the pickup event, and a state step counts the parts still shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartPickupErrand {
    pub actor_handler_rva: u32,
    pub pickup_event: u32,
    pub state_step_rva: u32,
    pub state_stage_offset: u64,
    pub counting_stage: i32,
    pub first_part_group: u16,
    pub last_part_group: u16,
}

pub const PART_PICKUP_ERRANDS: [PartPickupErrand; 1] = [PartPickupErrand {
    actor_handler_rva: 0x24_6890,
    pickup_event: 0x3C,
    state_step_rva: 0x30_17A0,
    state_stage_offset: 0x2C,
    counting_stage: 2,
    first_part_group: 10,
    last_part_group: 14,
}];

impl PartPickupErrand {
    pub fn has_part_group(&self, part_group: u16) -> bool {
        (self.first_part_group..=self.last_part_group).contains(&part_group)
    }
}

pub fn errand_of_handler(actor_handler_rva: u32) -> Option<&'static PartPickupErrand> {
    PART_PICKUP_ERRANDS
        .iter()
        .find(|errand| errand.actor_handler_rva == actor_handler_rva)
}

pub fn errand_of_state_step(state_step_rva: u32) -> Option<&'static PartPickupErrand> {
    PART_PICKUP_ERRANDS
        .iter()
        .find(|errand| errand.state_step_rva == state_step_rva)
}
