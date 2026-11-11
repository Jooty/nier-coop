#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedFire {
    Run,
    Drop { first_for_base: bool },
}

#[derive(Debug, Clone, Default)]
pub struct SharedEventsRan {
    phase: Option<u16>,
    ran: Vec<String>,
    drops_logged: Vec<String>,
}

impl SharedEventsRan {
    pub const fn new() -> Self {
        Self {
            phase: None,
            ran: Vec::new(),
            drops_logged: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn note_ran(&mut self, phase: u16, base: &str) {
        if self.phase != Some(phase) {
            self.reset();
            self.phase = Some(phase);
        }
        if !self.ran.iter().any(|known| known == base) {
            self.ran.push(base.to_owned());
        }
    }

    pub fn judge(&mut self, phase: u16, base: &str, led_by_partner: bool) -> SharedFire {
        let already_ran = self.phase == Some(phase) && self.ran.iter().any(|known| known == base);
        if led_by_partner || !already_ran {
            return SharedFire::Run;
        }
        let first_for_base = !self.drops_logged.iter().any(|known| known == base);
        if first_for_base {
            self.drops_logged.push(base.to_owned());
        }
        SharedFire::Drop { first_for_base }
    }
}
