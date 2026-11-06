use crate::effects::MAX_LIVE_PUPPET_EFFECTS;

#[derive(Debug, Clone, Default)]
pub struct SentEffects {
    handles: Vec<u32>,
}

impl SentEffects {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn track(&mut self, handle: u32) {
        if self.handles.len() >= MAX_LIVE_PUPPET_EFFECTS {
            self.handles.remove(0);
        }
        self.handles.push(handle);
    }

    /// Drops and returns the handles `has_ended` reports as ended.
    pub fn take_ended(&mut self, mut has_ended: impl FnMut(u32) -> bool) -> Vec<u32> {
        let (ended, live): (Vec<_>, Vec<_>) =
            self.handles.iter().partition(|handle| has_ended(**handle));
        self.handles = live;
        ended
    }

    pub fn clear(&mut self) {
        self.handles.clear();
    }

    pub fn len(&self) -> usize {
        self.handles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }
}
