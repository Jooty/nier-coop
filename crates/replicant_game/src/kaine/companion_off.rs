use crate::kaine::companion_off_fights::{CompanionOffFight, companion_off_fight_started_by};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionOffStep {
    Leave,
    Deactivate,
    /// The window closed: write her active byte back to what the scripts last left it.
    GiveBack {
        active: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionOffEvent {
    Opened { fight: &'static str, by: String },
    Closed { fight: &'static str, by: String },
}

/// The companion-off rule on one game: which fight window is open, what the scripts last left
/// her active byte as, and who turned her on since the last report.
#[derive(Debug, Clone, Default)]
pub struct CompanionOff {
    pack: String,
    open: Option<&'static CompanionOffFight>,
    holding: bool,
    scripts_left_her_active: Option<bool>,
    last_function: Option<String>,
    turned_on_by: Vec<String>,
}

/// What the rule saw since the last report, for the once-a-second line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionOffReport {
    pub fight: &'static str,
    pub turned_on_by: Vec<String>,
}

impl CompanionOff {
    pub const fn new() -> Self {
        Self {
            pack: String::new(),
            open: None,
            holding: false,
            scripts_left_her_active: None,
            last_function: None,
            turned_on_by: Vec::new(),
        }
    }

    pub fn open_fight(&self) -> Option<&'static str> {
        self.open.map(|fight| fight.name)
    }

    pub const fn holds_her(&self) -> bool {
        self.holding
    }

    pub fn enter_pack(&mut self, pack: &str) -> Option<CompanionOffEvent> {
        if self.pack == pack {
            return None;
        }
        pack.clone_into(&mut self.pack);
        self.last_function = None;
        let fight = self.open.take()?;
        Some(CompanionOffEvent::Closed {
            fight: fight.name,
            by: format!("leaving for {pack}"),
        })
    }

    pub fn function_ran(&mut self, name: &str) -> Option<CompanionOffEvent> {
        self.last_function = Some(name.to_owned());
        if let Some(fight) = self.open {
            if fight.end_functions.contains(&name) {
                self.open = None;
                return Some(CompanionOffEvent::Closed {
                    fight: fight.name,
                    by: name.to_owned(),
                });
            }
            return None;
        }
        let fight = companion_off_fight_started_by(name)?;
        self.open = Some(fight);
        Some(CompanionOffEvent::Opened {
            fight: fight.name,
            by: name.to_owned(),
        })
    }

    /// One tick: `active_now` is her active byte as found. While a window is open and a guest is
    /// in the session she is turned off; once it closes (or the guest leaves) she is given back.
    pub fn step(&mut self, guest_in_session: bool, active_now: bool) -> CompanionOffStep {
        if self.open.is_some() && guest_in_session {
            if self.scripts_left_her_active.is_none() {
                self.scripts_left_her_active = Some(active_now);
            } else if active_now {
                self.scripts_left_her_active = Some(true);
                let by = self
                    .last_function
                    .clone()
                    .unwrap_or_else(|| "no script function seen".to_owned());
                self.turned_on_by.push(by);
            }
            if active_now {
                self.holding = true;
                return CompanionOffStep::Deactivate;
            }
            return CompanionOffStep::Leave;
        }
        let was_holding = self.holding;
        let scripts_left = self.scripts_left_her_active.take();
        self.holding = false;
        if !was_holding || active_now {
            // never held, or the game turned her on itself after the window: it has her already
            return CompanionOffStep::Leave;
        }
        CompanionOffStep::GiveBack {
            active: scripts_left.unwrap_or(true),
        }
    }

    pub fn take_report(&mut self) -> Option<CompanionOffReport> {
        let fight = self.open?;
        Some(CompanionOffReport {
            fight: fight.name,
            turned_on_by: std::mem::take(&mut self.turned_on_by),
        })
    }
}
