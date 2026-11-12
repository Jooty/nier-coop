pub const MAX_HELD_BODIES: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeldBody {
    handle: u32,
    slot: u8,
    is_slot_body: bool,
}

#[derive(Debug, Clone, Default)]
pub struct HeldBodies {
    bodies: Vec<HeldBody>,
}

impl HeldBodies {
    pub const fn new() -> Self {
        Self { bodies: Vec::new() }
    }

    /// Holds a body; false when it is refused (the set is full). Holding a handle again
    /// replaces what was known about it.
    pub fn add(&mut self, handle: u32, slot: u8, is_slot_body: bool) -> bool {
        let held = HeldBody {
            handle,
            slot,
            is_slot_body,
        };
        if let Some(known) = self.bodies.iter_mut().find(|known| known.handle == handle) {
            *known = held;
            return true;
        }
        if self.bodies.len() >= MAX_HELD_BODIES {
            return false;
        }
        self.bodies.push(held);
        true
    }

    pub fn remove(&mut self, handle: u32) {
        self.bodies.retain(|known| known.handle != handle);
    }

    pub fn clear(&mut self) {
        self.bodies.clear();
    }

    pub fn holds(&self, handle: u32) -> bool {
        self.bodies.iter().any(|known| known.handle == handle)
    }

    pub fn slot_body(&self) -> Option<u32> {
        self.bodies
            .iter()
            .find(|known| known.is_slot_body)
            .map(|known| known.handle)
    }

    pub fn slot_of(&self, handle: u32) -> Option<u8> {
        self.bodies
            .iter()
            .find(|known| known.handle == handle)
            .map(|known| known.slot)
    }

    pub fn len(&self) -> usize {
        self.bodies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }
}
