mod kill_guard;
mod player_lock;
mod revive_display;
mod rewind_rules;
mod rules;

pub use revive_display::{
    LocalDownedLine, LocalDownedLineInputs, PartnerBadge, PartnerBadgeInputs, local_downed_line,
    partner_badge,
};

pub use rewind_rules::{
    BACKUP_SAVE_DATA_RVA, BACKUP_WAITS_FOR_PLAY_SECONDS, GAME_OVER_FLOW_WINDOW_MS,
    GAME_OVER_MESSAGE, GUEST_WAITS_FOR_HOST_GAME_OVER_SECONDS, HEAL_WATCH_GIVES_UP_SECONDS,
    HostGameOverGate, HostGameOverVerdict, MAP_MANAGER_RVA, OWN_DRIFT_HOLD_AFTER_REWIND_MS,
    RESTART_PHASE_RVA, RESTORE_SAVE_DATA_RVA, REWIND_WAITS_FOR_PLAY_SECONDS, RewindDrop,
    RewindRules,
};

pub use rules::{
    BLEED_OUT_SECONDS, DOWNED_HEALTH, DownedEffect, DownedRules, DownedState, HEARTBEAT_SECONDS,
    INCOMING_REVIVE_PROGRESS_STALE_SECONDS, MIN_REVIVED_HEALTH, REVIVE_HOLD_SECONDS,
    REVIVE_PROGRESS_PERCENT_MAX, REVIVE_PROGRESS_SEND_SECONDS, REVIVE_RANGE_UNITS,
    REVIVED_HEALTH_DIVISOR, StepInput, revived_health,
};

pub use kill_guard::{
    BlockedKillResponse, KillGuardInputs, blocked_kill_response, kill_is_blocked,
};

pub use player_lock::{
    PLAYER_CONTROL_DISABLE_NATIVE_RVAS, PLAYER_CONTROL_ENABLE_NATIVE_RVAS, PlayerLock,
    PlayerLockCall,
};
