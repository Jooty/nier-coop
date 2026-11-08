use crate::callback_gate::{
    CALLBACK_GATE_AFTER_DIALOG_CLOSE_MS, CALLBACK_GATE_GUEST_GIVE_UP_MS,
    CALLBACK_GATE_HOST_GRACE_MS, CALLBACK_GATE_PAIRING_WINDOW_MS,
};
use crate::story_calls::is_event_callback_name;
use crate::ui_scenes::PAUSE_MENU_CLOSE_MAX_REQUESTS_PER_SCENE_START;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateRole {
    Host,
    Guest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalFire {
    RunFirst,
    RunAfterPartner,
    RunReleased,
    RunAlone,
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartnerFire {
    Release {
        waited_ms: u64,
    },
    HostFiredFirst,
    HostWaits,
    /// The host started an event the guest's game has not reached; the guest, standing in the
    /// same place, starts it too. Whichever copy pops first runs, the other is a duplicate.
    FollowHost,
    Noted,
    Elsewhere,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateStep {
    GaveUp { name: String, waited_ms: u64 },
    CloseDialog { name: String, waited_ms: u64 },
    ClosePauseMenu { name: String, waited_ms: u64 },
    Inject { name: String, waited_ms: u64 },
}

#[derive(Debug, Clone, Default)]
pub struct CallbackGate {
    partner_fired: Vec<(String, u64)>,
    local_fired: Vec<(String, u64)>,
    held: Vec<(String, u64)>,
    wanted: Vec<Wanted>,
    let_pass: Vec<(String, u64)>,
    partner_elsewhere: bool,
}

#[derive(Debug, Clone)]
struct Wanted {
    name: String,
    since_ms: u64,
    dialog_closed_ms: Option<u64>,
    pause_menu_close_requests: u8,
}

impl CallbackGate {
    pub const fn new() -> Self {
        Self {
            partner_fired: Vec::new(),
            local_fired: Vec::new(),
            held: Vec::new(),
            wanted: Vec::new(),
            let_pass: Vec::new(),
            partner_elsewhere: false,
        }
    }

    pub fn note_partner_here(&mut self, partner_here: bool) {
        self.partner_elsewhere = !partner_here;
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn holds_any(&self) -> bool {
        !self.held.is_empty() || !self.wanted.is_empty()
    }

    pub fn release_held(&mut self, now_ms: u64) -> Vec<String> {
        self.wanted.clear();
        let names: Vec<String> = self.held.drain(..).map(|(name, _)| name).collect();
        for name in &names {
            self.let_pass.push((name.clone(), now_ms));
        }
        names
    }

    pub fn has_let_pass(&self, name: &str) -> bool {
        self.let_pass.iter().any(|(known, _)| known == name)
    }

    pub fn allow_next(&mut self, name: &str, now_ms: u64) {
        self.let_pass.push((name.to_owned(), now_ms));
    }

    pub fn local_fire(&mut self, role: GateRole, name: &str, now_ms: u64) -> LocalFire {
        self.prune(now_ms);
        if take_entry(&mut self.let_pass, name) {
            self.local_fired.push((name.to_owned(), now_ms));
            return LocalFire::RunReleased;
        }
        match role {
            GateRole::Host => {
                self.wanted.retain(|wanted| wanted.name != name);
                self.local_fired.push((name.to_owned(), now_ms));
                if take_entry(&mut self.partner_fired, name) {
                    LocalFire::RunAfterPartner
                } else {
                    LocalFire::RunFirst
                }
            }
            GateRole::Guest => {
                if take_entry(&mut self.partner_fired, name) {
                    LocalFire::RunAfterPartner
                } else if self.partner_elsewhere {
                    LocalFire::RunAlone
                } else {
                    if !self.held.iter().any(|(held, _)| held == name) {
                        self.held.push((name.to_owned(), now_ms));
                    }
                    LocalFire::Hold
                }
            }
        }
    }

    pub fn partner_fire(&mut self, role: GateRole, name: &str, now_ms: u64) -> PartnerFire {
        self.prune(now_ms);
        match role {
            GateRole::Guest => {
                if let Some(index) = self.held.iter().position(|(held, _)| held == name) {
                    let (_, since_ms) = self.held.remove(index);
                    self.let_pass.push((name.to_owned(), now_ms));
                    return PartnerFire::Release {
                        waited_ms: now_ms.saturating_sub(since_ms),
                    };
                }
                self.partner_fired.push((name.to_owned(), now_ms));
                if !self.partner_elsewhere && is_event_callback_name(name) {
                    return PartnerFire::FollowHost;
                }
                PartnerFire::Noted
            }
            GateRole::Host => {
                if take_entry(&mut self.local_fired, name) {
                    return PartnerFire::HostFiredFirst;
                }
                if self.partner_elsewhere {
                    return PartnerFire::Elsewhere;
                }
                if self.wanted.iter().any(|wanted| wanted.name == name) {
                    return PartnerFire::Noted;
                }
                self.wanted.push(Wanted {
                    name: name.to_owned(),
                    since_ms: now_ms,
                    dialog_closed_ms: None,
                    pause_menu_close_requests: 0,
                });
                PartnerFire::HostWaits
            }
        }
    }

    pub fn step(
        &mut self,
        role: GateRole,
        dialog_open: bool,
        pause_menu_open: bool,
        now_ms: u64,
    ) -> Vec<GateStep> {
        let mut steps = Vec::new();
        match role {
            GateRole::Guest => self.step_guest(now_ms, &mut steps),
            GateRole::Host => self.step_host(dialog_open, pause_menu_open, now_ms, &mut steps),
        }
        steps
    }

    fn step_guest(&mut self, now_ms: u64, steps: &mut Vec<GateStep>) {
        let mut released = Vec::new();
        self.held.retain(|(name, since_ms)| {
            let waited_ms = now_ms.saturating_sub(*since_ms);
            let gives_up = waited_ms >= CALLBACK_GATE_GUEST_GIVE_UP_MS;
            if gives_up {
                released.push((name.clone(), waited_ms));
            }
            !gives_up
        });
        for (name, waited_ms) in released {
            self.let_pass.push((name.clone(), now_ms));
            steps.push(GateStep::GaveUp { name, waited_ms });
        }
    }

    fn step_host(
        &mut self,
        dialog_open: bool,
        pause_menu_open: bool,
        now_ms: u64,
        steps: &mut Vec<GateStep>,
    ) {
        let mut due = Vec::new();
        for wanted in &mut self.wanted {
            let waited_ms = now_ms.saturating_sub(wanted.since_ms);
            let last_close_settled = wanted.dialog_closed_ms.is_none_or(|closed| {
                now_ms.saturating_sub(closed) >= CALLBACK_GATE_AFTER_DIALOG_CLOSE_MS
            });
            if pause_menu_open
                && wanted.pause_menu_close_requests < PAUSE_MENU_CLOSE_MAX_REQUESTS_PER_SCENE_START
                && last_close_settled
            {
                wanted.pause_menu_close_requests += 1;
                wanted.dialog_closed_ms = Some(now_ms);
                steps.push(GateStep::ClosePauseMenu {
                    name: wanted.name.clone(),
                    waited_ms,
                });
                continue;
            }
            if waited_ms < CALLBACK_GATE_HOST_GRACE_MS && wanted.pause_menu_close_requests == 0 {
                continue;
            }
            if dialog_open && wanted.dialog_closed_ms.is_none() {
                wanted.dialog_closed_ms = Some(now_ms);
                steps.push(GateStep::CloseDialog {
                    name: wanted.name.clone(),
                    waited_ms,
                });
                continue;
            }
            let settled = wanted.dialog_closed_ms.is_none_or(|closed| {
                now_ms.saturating_sub(closed) >= CALLBACK_GATE_AFTER_DIALOG_CLOSE_MS
            });
            if settled {
                due.push((wanted.name.clone(), waited_ms));
            }
        }
        for (name, waited_ms) in due {
            self.wanted.retain(|wanted| wanted.name != name);
            self.let_pass.push((name.clone(), now_ms));
            steps.push(GateStep::Inject { name, waited_ms });
        }
    }

    fn prune(&mut self, now_ms: u64) {
        let fresh = |entry: &(String, u64)| {
            now_ms.saturating_sub(entry.1) <= CALLBACK_GATE_PAIRING_WINDOW_MS
        };
        self.partner_fired.retain(fresh);
        self.local_fired.retain(fresh);
        self.let_pass.retain(fresh);
    }
}

fn take_entry(list: &mut Vec<(String, u64)>, name: &str) -> bool {
    let Some(index) = list.iter().position(|(entry, _)| entry == name) else {
        return false;
    };
    list.remove(index);
    true
}
