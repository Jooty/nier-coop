use crate::partner::BodySnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InputFeed {
    pub held: u32,
    pub pressed: u32,
    pub button_mask: u8,
    pub speed: f32,
    pub world_facing: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RemoteInput {
    held: u32,
    pressed_not_yet_fed: u32,
    button_mask: u8,
    speed: f32,
    world_facing: f32,
}

impl InputFeed {
    pub fn speed_squared(&self) -> f32 {
        self.speed * self.speed
    }
}

impl RemoteInput {
    pub fn receive(&mut self, snapshot: &BodySnapshot) {
        // A button that became held counts as pressed even when the message with its edge was lost.
        let newly_held = snapshot.held & !self.held;
        self.pressed_not_yet_fed |= snapshot.pressed | newly_held;
        self.held = snapshot.held;
        self.button_mask = snapshot.button_mask;
        self.speed = snapshot.speed;
        self.world_facing = snapshot.world_facing;
    }

    /// The feed for the next controller update; the pressed bits are handed out once.
    pub fn take_feed(&mut self) -> InputFeed {
        InputFeed {
            held: self.held,
            pressed: core::mem::take(&mut self.pressed_not_yet_fed),
            button_mask: self.button_mask,
            speed: self.speed,
            world_facing: self.world_facing,
        }
    }
}
