use crate::partner::SNAPSHOT_SEND_INTERVAL_MS;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SendPacer {
    last_sent_ms: Option<u64>,
    pressed_since_last_send: u32,
}

impl SendPacer {
    /// Takes one frame's pressed bits; when a send is due, returns every bit pressed since the last one.
    pub fn observe(&mut self, now_ms: u64, pressed: u32) -> Option<u32> {
        self.pressed_since_last_send |= pressed;
        let due = self
            .last_sent_ms
            .is_none_or(|sent| now_ms.saturating_sub(sent) >= SNAPSHOT_SEND_INTERVAL_MS);
        if !due {
            return None;
        }
        self.last_sent_ms = Some(now_ms);
        Some(core::mem::take(&mut self.pressed_since_last_send))
    }
}
