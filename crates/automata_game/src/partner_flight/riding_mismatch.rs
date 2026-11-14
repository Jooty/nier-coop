use crate::partner_flight::{
    RIDING_MISMATCH_LOGGED_AFTER_MS, riding_agrees_again_line, riding_mismatch_line,
};

/// Watches whether the hero on this game and the partner body differ in being in a flight unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RidingMismatch {
    since_ms: Option<u64>,
    logged: bool,
}

impl RidingMismatch {
    pub fn observe(
        &mut self,
        now_ms: u64,
        hero_in_a_unit: bool,
        body_rides: bool,
    ) -> Option<String> {
        if hero_in_a_unit == body_rides {
            let since = self.since_ms.take();
            let was_logged = core::mem::take(&mut self.logged);
            return since
                .filter(|_| was_logged)
                .map(|since| riding_agrees_again_line(now_ms.saturating_sub(since)));
        }
        let since = *self.since_ms.get_or_insert(now_ms);
        if !self.logged && now_ms.saturating_sub(since) >= RIDING_MISMATCH_LOGGED_AFTER_MS {
            self.logged = true;
            return Some(riding_mismatch_line(hero_in_a_unit, body_rides));
        }
        None
    }

    pub fn forget(&mut self) {
        *self = Self::default();
    }
}
