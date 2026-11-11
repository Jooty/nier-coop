use replicant_proto::{AttackGaugeOpen, AttackGaugeState, EnemyId};

use crate::attack_gauge::HOST_GAUGE_RESEND_INTERVAL_MS;

/// The host's side of the gauge mirror: numbers each opening and decides when the state goes to
/// the guest (at once when it opens, closes or a node changes, otherwise every resend interval).
#[derive(Clone, Debug, Default)]
pub struct HostGaugeWatch {
    serial: u32,
    shown: Option<Shown>,
}

#[derive(Clone, Debug)]
struct Shown {
    phase: u16,
    boss: EnemyId,
    partner: Option<EnemyId>,
    open: AttackGaugeOpen,
    sent_at_ms: u64,
}

impl HostGaugeWatch {
    pub const fn new() -> Self {
        Self {
            serial: 0,
            shown: None,
        }
    }

    pub fn serial(&self) -> Option<u32> {
        self.shown.as_ref().map(|_| self.serial)
    }

    pub fn open(&self) -> Option<&AttackGaugeOpen> {
        self.shown.as_ref().map(|shown| &shown.open)
    }

    pub fn observe(
        &mut self,
        now_ms: u64,
        phase: u16,
        open: Option<AttackGaugeOpen>,
    ) -> Option<AttackGaugeState> {
        let Some(open) = open else {
            let closed = self.shown.take()?;
            return Some(AttackGaugeState {
                phase: closed.phase,
                serial: self.serial,
                open: None,
            });
        };
        let same_opening = self.shown.as_ref().is_some_and(|shown| {
            shown.phase == phase
                && shown.boss == open.boss
                && shown.partner == open.partner
                && open.remaining_seconds <= shown.open.remaining_seconds
        });
        if !same_opening {
            self.serial = self.serial.wrapping_add(1);
        } else if let Some(shown) = &self.shown {
            let nodes_same = shown.open.nodes == open.nodes;
            let due = now_ms.saturating_sub(shown.sent_at_ms) >= HOST_GAUGE_RESEND_INTERVAL_MS;
            if nodes_same && !due {
                return None;
            }
        }
        self.shown = Some(Shown {
            phase,
            boss: open.boss,
            partner: open.partner,
            open: open.clone(),
            sent_at_ms: now_ms,
        });
        Some(AttackGaugeState {
            phase,
            serial: self.serial,
            open: Some(open),
        })
    }

    pub fn reset(&mut self) {
        self.shown = None;
    }
}
