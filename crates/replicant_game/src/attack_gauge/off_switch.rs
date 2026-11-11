use crate::attack_gauge::{
    ATTACK_GAUGE_OFF_FILE_NAME, ATTACK_GAUGE_SWITCH_CHECK_INTERVAL_MS, ATTACK_GAUGE_SYNC_OFF_TEXT,
    ATTACK_GAUGE_SYNC_ON_TEXT,
};

/// The off switch for the gauge sync: a file beside the mod's `logs` directory. It is looked at
/// when a gauge opens and at most once a second while one is open; once off, the sync stays off
/// until the next opening finds the file gone.
#[derive(Clone, Debug, Default)]
pub struct GaugeOffSwitch {
    off: bool,
    gauge_was_open: bool,
    checked_at_ms: Option<u64>,
    logged_phase: Option<u16>,
}

impl GaugeOffSwitch {
    pub const fn new() -> Self {
        Self {
            off: false,
            gauge_was_open: false,
            checked_at_ms: None,
            logged_phase: None,
        }
    }

    pub fn observe(
        &mut self,
        now_ms: u64,
        gauge_open: bool,
        file_exists: impl FnOnce() -> bool,
    ) -> bool {
        let opening = gauge_open && !self.gauge_was_open;
        self.gauge_was_open = gauge_open;
        if !gauge_open {
            return self.off;
        }
        if opening {
            self.off = file_exists();
            self.checked_at_ms = Some(now_ms);
        } else if self.checked_at_ms.is_none_or(|checked| {
            now_ms.saturating_sub(checked) >= ATTACK_GAUGE_SWITCH_CHECK_INTERVAL_MS
        }) {
            self.off |= file_exists();
            self.checked_at_ms = Some(now_ms);
        }
        self.off
    }

    pub fn is_off(&self) -> bool {
        self.off
    }

    pub fn should_log_off(&mut self, phase: u16) -> bool {
        if !self.off || self.logged_phase == Some(phase) {
            return false;
        }
        self.logged_phase = Some(phase);
        true
    }
}

/// Where the off switch file sits for a mod writing its log to `log_path`: in the parent of the
/// `logs` directory, or beside the log when it is not inside one. Takes both path separators.
pub fn attack_gauge_off_file(log_path: &str) -> String {
    let separator = if log_path.contains('\\') { '\\' } else { '/' };
    let parent = |path: &str| path.rfind(['/', '\\']).map(|at| path[..at].to_owned());
    let Some(log_dir) = parent(log_path) else {
        return ATTACK_GAUGE_OFF_FILE_NAME.to_owned();
    };
    let last = log_dir.rsplit(['/', '\\']).next().unwrap_or("");
    let data_dir = if last == "logs" {
        parent(&log_dir).unwrap_or_default()
    } else {
        log_dir
    };
    format!("{data_dir}{separator}{ATTACK_GAUGE_OFF_FILE_NAME}")
}

pub fn attack_gauge_boot_text(off: bool) -> &'static str {
    if off {
        ATTACK_GAUGE_SYNC_OFF_TEXT
    } else {
        ATTACK_GAUGE_SYNC_ON_TEXT
    }
}
