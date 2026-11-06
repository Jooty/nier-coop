pub const FRAME_KEY_BYTES: usize = 48;
pub const FRAME_RING_SLOTS: usize = 8;

pub type FrameKey = [u8; FRAME_KEY_BYTES];

#[derive(Debug, Clone)]
pub struct FrameRing<T: Copy> {
    slots: [Option<(FrameKey, T)>; FRAME_RING_SLOTS],
    next: usize,
}

impl<T: Copy> FrameRing<T> {
    pub const fn new() -> Self {
        Self {
            slots: [None; FRAME_RING_SLOTS],
            next: 0,
        }
    }

    pub fn push(&mut self, key: FrameKey, value: T) {
        self.slots[self.next] = Some((key, value));
        self.next = (self.next + 1) % FRAME_RING_SLOTS;
    }

    pub fn find(&self, key: &FrameKey) -> Option<T> {
        self.find_with_age(key).map(|(value, _)| value)
    }

    pub fn find_with_age(&self, key: &FrameKey) -> Option<(T, usize)> {
        (0..FRAME_RING_SLOTS).find_map(|back| {
            let (stored, value) = self.nth_back_entry(back)?;
            (stored == *key).then_some((value, back))
        })
    }

    pub fn nth_back(&self, back: usize) -> Option<T> {
        self.nth_back_entry(back).map(|(_, value)| value)
    }

    fn nth_back_entry(&self, back: usize) -> Option<(FrameKey, T)> {
        if back >= FRAME_RING_SLOTS {
            return None;
        }
        self.slots[(self.next + FRAME_RING_SLOTS - 1 - back) % FRAME_RING_SLOTS]
    }
}

impl<T: Copy> Default for FrameRing<T> {
    fn default() -> Self {
        Self::new()
    }
}
