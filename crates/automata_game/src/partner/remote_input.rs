use crate::partner::BodySnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InputFeed {
    pub held: u32,
    pub pressed: u32,
    pub speed: f32,
    pub world_facing: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RemoteInput {
    held: u32,
    pressed_not_yet_fed: u32,
    speed: f32,
    world_facing: f32,
}

impl InputFeed {
    pub fn speed_squared(&self) -> f32 {
        self.speed * self.speed
    }

    /// No buttons and no speed, facing kept: what a body gets while the other player is silent.
    pub fn stopped(self) -> Self {
        Self {
            world_facing: self.world_facing,
            ..Self::default()
        }
    }

    /// Speed and facing only: what a body driven by state gets.
    pub fn without_buttons(self) -> Self {
        Self {
            held: 0,
            pressed: 0,
            ..self
        }
    }
}

impl RemoteInput {
    pub fn receive(&mut self, snapshot: &BodySnapshot) {
        // A button that became held counts as pressed even when the message with its edge was lost.
        let newly_held = snapshot.held & !self.held;
        self.pressed_not_yet_fed |= snapshot.pressed | newly_held;
        self.held = snapshot.held;
        self.speed = snapshot.speed;
        self.world_facing = snapshot.world_facing;
    }

    /// Call on every tick that feeds nothing, so presses do not pile up into one update.
    pub fn drop_presses(&mut self) {
        self.pressed_not_yet_fed = 0;
    }

    /// The feed for the next controller update; the pressed bits are handed out once.
    pub fn take_feed(&mut self) -> InputFeed {
        InputFeed {
            held: self.held,
            pressed: core::mem::take(&mut self.pressed_not_yet_fed),
            speed: self.speed,
            world_facing: self.world_facing,
        }
    }
}
