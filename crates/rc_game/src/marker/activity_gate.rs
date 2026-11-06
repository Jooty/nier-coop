use rc_proto::PartnerActivity;

use crate::marker::MENU_STALL_GRACE_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActivityGate {
    last_motion: Option<(i32, u32)>,
    last_progress_ms: u64,
}

impl ActivityGate {
    pub const fn new() -> Self {
        Self {
            last_motion: None,
            last_progress_ms: 0,
        }
    }

    pub fn settle(
        &mut self,
        raw: PartnerActivity,
        motion: Option<(i32, f32)>,
        now_ms: u64,
    ) -> PartnerActivity {
        let key = motion.map(|(index, frame)| (index, frame.to_bits()));
        if key != self.last_motion {
            self.last_motion = key;
            self.last_progress_ms = now_ms;
        }
        let still_moving = now_ms.saturating_sub(self.last_progress_ms) < MENU_STALL_GRACE_MS;
        if raw == PartnerActivity::InMenu && motion.is_some() && still_moving {
            PartnerActivity::Playing
        } else {
            raw
        }
    }
}
