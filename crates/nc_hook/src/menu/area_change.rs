use crate::menu::{AreaChangeRefusal, AreaEntry};

pub const AREA_CHANGE_TIMEOUT_MS: u64 = 15_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AreaChangeOutcome {
    Changed(String),
    TimedOut(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingChange {
    target: AreaEntry,
    requested_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum LastResult {
    #[default]
    Nothing,
    Waiting(String),
    Changed(String),
    Refused(String),
    TimedOut(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AreaChangeTracker {
    pending: Option<PendingChange>,
    last: LastResult,
}

impl AreaChangeTracker {
    pub fn request(
        &mut self,
        target: &AreaEntry,
        current: Option<&AreaEntry>,
        blocker: Option<AreaChangeRefusal>,
        now_ms: u64,
    ) -> Result<(), AreaChangeRefusal> {
        let refusal = blocker.or_else(|| {
            current
                .is_some_and(|here| here.id == target.id)
                .then_some(AreaChangeRefusal::AlreadyThere)
        });
        if let Some(refusal) = refusal {
            self.pending = None;
            self.last = LastResult::Refused(refusal.to_string());
            return Err(refusal);
        }
        self.pending = Some(PendingChange {
            target: target.clone(),
            requested_at_ms: now_ms,
        });
        self.last = LastResult::Waiting(target.name.clone());
        Ok(())
    }

    pub fn game_refused(&mut self, reason: &str) {
        self.pending = None;
        self.last = LastResult::Refused(reason.to_owned());
    }

    pub fn observe(
        &mut self,
        current: Option<&AreaEntry>,
        now_ms: u64,
    ) -> Option<AreaChangeOutcome> {
        let pending = self.pending.as_ref()?;
        if current.is_some_and(|here| here.id == pending.target.id) {
            let name = pending.target.name.clone();
            self.pending = None;
            self.last = LastResult::Changed(name.clone());
            return Some(AreaChangeOutcome::Changed(name));
        }
        if now_ms.saturating_sub(pending.requested_at_ms) >= AREA_CHANGE_TIMEOUT_MS {
            let name = pending.target.name.clone();
            self.pending = None;
            self.last = LastResult::TimedOut(name.clone());
            return Some(AreaChangeOutcome::TimedOut(name));
        }
        None
    }

    pub fn is_waiting(&self) -> bool {
        self.pending.is_some()
    }

    pub fn message(&self) -> String {
        match &self.last {
            LastResult::Nothing => String::new(),
            LastResult::Waiting(name) => format!("changing to {name}"),
            LastResult::Changed(name) => format!("changed to {name}"),
            LastResult::Refused(reason) => format!("not changed: {reason}"),
            LastResult::TimedOut(name) => {
                format!(
                    "not changed to {name}: no change after {} s",
                    AREA_CHANGE_TIMEOUT_MS / 1000
                )
            }
        }
    }
}

impl AreaChangeOutcome {
    pub fn log_line(&self) -> String {
        match self {
            Self::Changed(name) => format!("teleport: area changed to {name}"),
            Self::TimedOut(name) => format!(
                "teleport: no change to {name} after {} s",
                AREA_CHANGE_TIMEOUT_MS / 1000
            ),
        }
    }
}
