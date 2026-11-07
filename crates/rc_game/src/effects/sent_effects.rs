use crate::effects::{MAX_LASTING_SENT_EFFECTS, MAX_LIVE_PUPPET_EFFECTS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LastingEffect {
    handle: u32,
    anchor: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct SentEffects {
    ordinary: Vec<u32>,
    lasting: Vec<LastingEffect>,
}

impl SentEffects {
    pub fn new() -> Self {
        Self::default()
    }

    /// Remembers a short effect; the oldest is handed back when the list is full.
    pub fn track(&mut self, handle: u32) -> Option<u32> {
        let evicted =
            (self.ordinary.len() >= MAX_LIVE_PUPPET_EFFECTS).then(|| self.ordinary.remove(0));
        self.ordinary.push(handle);
        evicted
    }

    /// Remembers a looping or power effect in its own list, with the actor that plays it.
    pub fn track_lasting(&mut self, handle: u32, anchor: Option<u64>) -> Option<u32> {
        let evicted =
            (self.lasting.len() >= MAX_LASTING_SENT_EFFECTS).then(|| self.lasting.remove(0).handle);
        self.lasting.push(LastingEffect { handle, anchor });
        evicted
    }

    /// Drops and returns the handles `has_ended` reports as ended.
    pub fn take_ended(&mut self, mut has_ended: impl FnMut(u32) -> bool) -> Vec<u32> {
        let (mut ended, live): (Vec<_>, Vec<_>) =
            self.ordinary.iter().partition(|handle| has_ended(**handle));
        self.ordinary = live;
        let (ended_lasting, live_lasting): (Vec<_>, Vec<_>) = self
            .lasting
            .iter()
            .partition(|effect| has_ended(effect.handle));
        self.lasting = live_lasting;
        ended.extend(ended_lasting.into_iter().map(|effect| effect.handle));
        ended
    }

    /// Drops and returns the lasting effects whose actor `is_gone` reports as gone.
    pub fn take_anchor_gone(&mut self, mut is_gone: impl FnMut(u64) -> bool) -> Vec<u32> {
        let (gone, kept): (Vec<_>, Vec<_>) = self
            .lasting
            .iter()
            .partition(|effect| effect.anchor.is_some_and(&mut is_gone));
        self.lasting = kept;
        gone.into_iter().map(|effect| effect.handle).collect()
    }

    pub fn take_all(&mut self) -> Vec<u32> {
        let mut all = std::mem::take(&mut self.ordinary);
        all.extend(self.lasting.drain(..).map(|effect| effect.handle));
        all
    }

    pub fn len(&self) -> usize {
        self.ordinary.len() + self.lasting.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ordinary.is_empty() && self.lasting.is_empty()
    }
}
