use super::RUNNER_LOG_INTERVAL_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseCall {
    On,
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRunnerSnapshot {
    pub pause_mask: u32,
    pub queued: u32,
    pub ready: u32,
    pub last_pop: Option<(String, u64)>,
}

#[derive(Debug, Default)]
pub struct ScriptRunnerLogGate {
    last_state_ms: Option<u64>,
    last_empty_pop_ms: Option<u64>,
}

impl ScriptRunnerSnapshot {
    pub const fn is_idle(&self) -> bool {
        self.pause_mask == 0 && self.queued == 0
    }
}

impl ScriptRunnerLogGate {
    pub const fn new() -> Self {
        Self {
            last_state_ms: None,
            last_empty_pop_ms: None,
        }
    }

    pub fn state_due(&mut self, idle: bool, now_ms: u64) -> bool {
        if idle {
            self.last_state_ms = None;
            return false;
        }
        Self::due(&mut self.last_state_ms, now_ms)
    }

    pub fn empty_pop_due(&mut self, now_ms: u64) -> bool {
        Self::due(&mut self.last_empty_pop_ms, now_ms)
    }

    fn due(last: &mut Option<u64>, now_ms: u64) -> bool {
        if last.is_some_and(|at| now_ms.saturating_sub(at) < RUNNER_LOG_INTERVAL_MS) {
            return false;
        }
        *last = Some(now_ms);
        true
    }
}

pub fn format_runner_state(snapshot: &ScriptRunnerSnapshot) -> String {
    format!(
        "scripts: pause {:#x}, queue {}, ready {}, {}",
        snapshot.pause_mask,
        snapshot.queued,
        snapshot.ready,
        last_pop_text(snapshot.last_pop.as_ref())
    )
}

pub fn format_empty_pop(snapshot: &ScriptRunnerSnapshot) -> String {
    format!(
        "scripts: the game's pop returned nothing while {} queued ({} ready), pause {:#x}, {}",
        snapshot.queued,
        snapshot.ready,
        snapshot.pause_mask,
        last_pop_text(snapshot.last_pop.as_ref())
    )
}

pub fn format_pause_call(
    call: PauseCall,
    mask_before: u32,
    mask_after: u32,
    running: &str,
) -> String {
    let name = match call {
        PauseCall::On => "PauseOn",
        PauseCall::Off => "PauseOff",
    };
    let running = if running.is_empty() {
        "unknown"
    } else {
        running
    };
    format!("scripts: {name} by {running} (pause {mask_before:#x} -> {mask_after:#x})")
}

fn last_pop_text(last_pop: Option<&(String, u64)>) -> String {
    match last_pop {
        Some((name, age_ms)) => format!("last pop {name} {}s ago", age_ms / 1_000),
        None => "no pop yet".to_owned(),
    }
}
