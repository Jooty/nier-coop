use crate::kaine::companion_off_fights::{
    CompanionOffFight, companion_off_fight_approached_in, companion_off_fight_started_by,
};
use crate::kaine::{COMPANION_LENT_AFTER_A_CALL_MS, COMPANION_LENT_AFTER_A_SCENE_MS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionOffStep {
    Leave,
    Deactivate,
    /// The window closed: write her active byte back to what the scripts last left it.
    GiveBack {
        active: bool,
    },
    /// She is lent to the game: write her active byte 1 when `write_active`, and leave her be.
    Lend {
        write_active: bool,
        why: CompanionLendReason,
    },
    /// The lend is over: she is turned off again when `deactivate`.
    LendOver {
        deactivate: bool,
        why: CompanionLendReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionLendReason {
    Scene,
    Call(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionOffEvent {
    Opened { fight: &'static str, by: String },
    Closed { fight: &'static str, by: String },
}

/// One ghost tick as the rule sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanionOffTick {
    pub guest_in_session: bool,
    pub active_now: bool,
    pub event_scene: bool,
    pub now_ms: u64,
}

/// A script function that addresses her popped inside the window: write her active byte as
/// below before it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanionCallLend {
    pub fight: &'static str,
    pub write_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Lend {
    why: CompanionLendReason,
    until_ms: Option<u64>,
    written: bool,
}

/// The companion-off rule on one game: which fight window is open, what the scripts last left
/// her active byte as, who turned her on since the last report, and whether she is lent back.
#[derive(Debug, Clone, Default)]
pub struct CompanionOff {
    pack: String,
    open: Option<&'static CompanionOffFight>,
    holding: bool,
    scripts_left_her_active: Option<bool>,
    last_function: Option<String>,
    turned_on_by: Vec<String>,
    in_scene: bool,
    lend: Option<Lend>,
}

/// What the rule saw since the last report, for the once-a-second line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionOffReport {
    pub fight: &'static str,
    pub turned_on_by: Vec<String>,
    pub lent: Option<CompanionLendReason>,
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
            in_scene: false,
            lend: None,
        }
    }

    pub fn open_fight(&self) -> Option<&'static str> {
        self.open.map(|fight| fight.name)
    }

    pub const fn holds_her(&self) -> bool {
        self.holding
    }

    pub fn lent_for(&self) -> Option<&CompanionLendReason> {
        self.lend.as_ref().map(|lend| &lend.why)
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

    /// Whether the current pack holds a fight's approach, so the story flags are worth reading.
    pub fn approach_is_here(&self) -> bool {
        self.open.is_none() && companion_off_fight_approached_in(&self.pack).is_some()
    }

    /// The story flags, read each tick in a pack with an approach: they open the window from
    /// the area's start while the fight's stretch is on.
    pub fn story_flags(&mut self, flag_on: impl Fn(u16) -> bool) -> Option<CompanionOffEvent> {
        if self.open.is_some() {
            return None;
        }
        let fight = companion_off_fight_approached_in(&self.pack)?;
        let approach = fight.approach;
        if !flag_on(approach.on_game_flag) || flag_on(approach.off_game_flag) {
            return None;
        }
        self.open = Some(fight);
        Some(CompanionOffEvent::Opened {
            fight: fight.name,
            by: format!(
                "the story flags (g{} on, g{} off) in {}",
                approach.on_game_flag, approach.off_game_flag, approach.pack
            ),
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

    /// After [`Self::function_ran`], at the pop, before `name` runs: when it addresses her inside
    /// an open window with a guest in the session, she is lent to the game for it and a moment
    /// after. `tick` is the game as found at the pop.
    pub fn function_addresses_her(
        &mut self,
        name: &str,
        tick: CompanionOffTick,
    ) -> Option<CompanionCallLend> {
        let fight = self.open?;
        if !tick.guest_in_session || !fight.functions_that_address_her.contains(&name) {
            return None;
        }
        let now_ms = tick.now_ms;
        let write_active = *self.scripts_left_her_active.get_or_insert(tick.active_now);
        let until_ms = now_ms + COMPANION_LENT_AFTER_A_CALL_MS;
        match &mut self.lend {
            Some(lend) if lend.why == CompanionLendReason::Scene && lend.until_ms.is_none() => {}
            Some(lend) => {
                lend.why = CompanionLendReason::Call(name.to_owned());
                lend.until_ms = Some(lend.until_ms.map_or(until_ms, |at| at.max(until_ms)));
                lend.written = true;
            }
            None => {
                self.lend = Some(Lend {
                    why: CompanionLendReason::Call(name.to_owned()),
                    until_ms: Some(until_ms),
                    written: true,
                });
            }
        }
        Some(CompanionCallLend {
            fight: fight.name,
            write_active,
        })
    }

    /// One tick. While a window is open and a guest is in the session she is turned off, except
    /// during an event scene and a moment after it, or after a call that addresses her; once the
    /// window closes (or the guest leaves) she is given back.
    pub fn step(&mut self, tick: CompanionOffTick) -> CompanionOffStep {
        if self.open.is_some() && tick.guest_in_session {
            if let Some(step) = self.lend_step(tick) {
                return step;
            }
            if self.scripts_left_her_active.is_none() {
                self.scripts_left_her_active = Some(tick.active_now);
            } else if tick.active_now {
                self.scripts_left_her_active = Some(true);
                let by = self
                    .last_function
                    .clone()
                    .unwrap_or_else(|| "no script function seen".to_owned());
                self.turned_on_by.push(by);
            }
            if tick.active_now {
                self.holding = true;
                return CompanionOffStep::Deactivate;
            }
            return CompanionOffStep::Leave;
        }
        self.lend = None;
        self.in_scene = false;
        let was_holding = self.holding;
        let scripts_left = self.scripts_left_her_active.take();
        self.holding = false;
        if !was_holding || tick.active_now {
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
            lent: self.lend.as_ref().map(|lend| lend.why.clone()),
        })
    }

    fn lend_step(&mut self, tick: CompanionOffTick) -> Option<CompanionOffStep> {
        if tick.event_scene && !self.in_scene {
            self.in_scene = true;
            self.lend = Some(Lend {
                why: CompanionLendReason::Scene,
                until_ms: None,
                written: false,
            });
        } else if !tick.event_scene && self.in_scene {
            self.in_scene = false;
            if let Some(lend) = &mut self.lend {
                let until_ms = tick.now_ms + COMPANION_LENT_AFTER_A_SCENE_MS;
                lend.until_ms = Some(lend.until_ms.map_or(until_ms, |at| at.max(until_ms)));
            }
        }
        let lend = self.lend.as_mut()?;
        let scripts_left = *self.scripts_left_her_active.get_or_insert(tick.active_now);
        if lend.until_ms.is_some_and(|until| tick.now_ms >= until) {
            let why = lend.why.clone();
            self.lend = None;
            if tick.active_now {
                self.holding = true;
            }
            return Some(CompanionOffStep::LendOver {
                deactivate: tick.active_now,
                why,
            });
        }
        if !lend.written {
            lend.written = true;
            let write_active = scripts_left && !tick.active_now;
            return Some(CompanionOffStep::Lend {
                write_active,
                why: lend.why.clone(),
            });
        }
        // while she is lent, what the game does to her is what the scripts left her as
        self.scripts_left_her_active = Some(tick.active_now);
        Some(CompanionOffStep::Leave)
    }
}
