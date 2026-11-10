use crate::blocks::BLOCK_HELD_REPEAT_INTERVAL_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoldAnnouncement {
    pub phase: u16,
    pub id: u16,
    pub held: bool,
}

#[derive(Debug, Default)]
pub struct HoldAnnouncer {
    current: Option<(u16, u16, u64)>,
}

impl HoldAnnouncer {
    pub fn observe(&mut self, now_ms: u64, phase: u16, held: Option<u16>) -> Vec<HoldAnnouncement> {
        let mut out = Vec::new();
        if let Some((old_phase, old_id, _)) = self.current
            && (held != Some(old_id) || old_phase != phase)
        {
            out.push(HoldAnnouncement {
                phase: old_phase,
                id: old_id,
                held: false,
            });
            self.current = None;
        }
        let Some(id) = held else {
            return out;
        };
        match &mut self.current {
            None => {
                self.current = Some((phase, id, now_ms));
                out.push(HoldAnnouncement {
                    phase,
                    id,
                    held: true,
                });
            }
            Some((_, _, last_sent)) => {
                if now_ms.saturating_sub(*last_sent) >= BLOCK_HELD_REPEAT_INTERVAL_MS {
                    *last_sent = now_ms;
                    out.push(HoldAnnouncement {
                        phase,
                        id,
                        held: true,
                    });
                }
            }
        }
        out
    }
}
