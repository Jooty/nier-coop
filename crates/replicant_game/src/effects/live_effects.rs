use crate::effects::{MAX_LIVE_PUPPET_EFFECTS, PUPPET_EFFECT_MAX_LIFETIME_MS};

#[derive(Debug, Clone, Default)]
pub struct LiveEffects {
    spawned: Vec<(u32, u64, u32)>,
}

impl LiveEffects {
    pub fn new() -> Self {
        Self::default()
    }

    /// Remembers an effect the puppet started; the oldest is returned when the list is full.
    pub fn track(&mut self, handle: u32, sender_handle: u32, now_ms: u64) -> Option<u32> {
        let evicted =
            (self.spawned.len() >= MAX_LIVE_PUPPET_EFFECTS).then(|| self.spawned.remove(0).0);
        self.spawned.push((handle, now_ms, sender_handle));
        evicted
    }

    /// Takes the effects that have outlived the puppet effect lifetime, as (puppet handle, sender handle).
    pub fn take_expired(&mut self, now_ms: u64) -> Vec<(u32, u32)> {
        let (expired, kept): (Vec<_>, Vec<_>) =
            self.spawned.iter().copied().partition(|&(_, started, _)| {
                now_ms.saturating_sub(started) >= PUPPET_EFFECT_MAX_LIFETIME_MS
            });
        self.spawned = kept;
        expired
            .into_iter()
            .map(|(handle, _, sender)| (handle, sender))
            .collect()
    }

    /// Takes the puppet effect that mirrors the effect the sender knows by `sender_handle`.
    pub fn take_by_sender(&mut self, sender_handle: u32) -> Option<u32> {
        let position = self
            .spawned
            .iter()
            .position(|&(_, _, sender)| sender == sender_handle)?;
        Some(self.spawned.remove(position).0)
    }

    /// The puppet effect that mirrors `sender_handle`, left in the list.
    pub fn own_handle_of(&self, sender_handle: u32) -> Option<u32> {
        self.spawned
            .iter()
            .find(|&&(_, _, sender)| sender == sender_handle)
            .map(|&(handle, _, _)| handle)
    }

    pub fn take_all(&mut self) -> Vec<u32> {
        self.spawned
            .drain(..)
            .map(|(handle, _, _)| handle)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.spawned.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spawned.is_empty()
    }
}
