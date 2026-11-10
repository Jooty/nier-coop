use crate::scene_follow::{
    SCENE_FOLLOW_AFTER_DIALOG_CLOSE_MS, SCENE_FOLLOW_ALREADY_ENTERED_MS, SCENE_FOLLOW_GIVE_UP_MS,
    SCENE_FOLLOW_GRACE_MS,
};
use crate::ui_scenes::PAUSE_MENU_CLOSE_MAX_REQUESTS_PER_SCENE_START;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowStep {
    Idle,
    Wait,
    Followed {
        waited_ms: u64,
    },
    CloseDialog {
        waited_ms: u64,
    },
    ClosePauseMenu {
        waited_ms: u64,
    },
    Inject {
        waited_ms: u64,
        callbacks: Vec<String>,
    },
    GaveUp {
        waited_ms: u64,
    },
}

#[derive(Debug, Clone, Default)]
pub struct SceneFollowWatch {
    pending: Option<Pending>,
    last_entry_ms: Option<u64>,
    was_in_scene: bool,
    host_in_scene: bool,
}

#[derive(Debug, Clone)]
struct Pending {
    callbacks: Vec<String>,
    received_ms: u64,
    dialog_closed_ms: Option<u64>,
    pause_menu_close_requests: u8,
    injected: bool,
}

impl SceneFollowWatch {
    pub const fn new() -> Self {
        Self {
            pending: None,
            last_entry_ms: None,
            was_in_scene: false,
            host_in_scene: false,
        }
    }

    pub fn set_host_in_scene(&mut self, host_in_scene: bool) {
        self.host_in_scene = host_in_scene;
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub fn receive(&mut self, callbacks: Vec<String>, now_ms: u64) {
        self.pending = Some(Pending {
            callbacks,
            received_ms: now_ms,
            dialog_closed_ms: None,
            pause_menu_close_requests: 0,
            injected: false,
        });
    }

    pub fn step(
        &mut self,
        local_in_scene: bool,
        dialog_open: bool,
        pause_menu_open: bool,
        now_ms: u64,
    ) -> FollowStep {
        if local_in_scene && !self.was_in_scene {
            self.last_entry_ms = Some(now_ms);
        }
        self.was_in_scene = local_in_scene;
        let Some(pending) = self.pending.as_mut() else {
            return FollowStep::Idle;
        };
        let waited_ms = now_ms.saturating_sub(pending.received_ms);
        let entered_just_before = self.last_entry_ms.is_some_and(|entered| {
            entered + SCENE_FOLLOW_ALREADY_ENTERED_MS >= pending.received_ms
        });
        if entered_just_before || (local_in_scene && waited_ms > 0) {
            self.pending = None;
            return FollowStep::Followed { waited_ms };
        }
        if waited_ms >= SCENE_FOLLOW_GIVE_UP_MS {
            self.pending = None;
            return FollowStep::GaveUp { waited_ms };
        }
        let last_close_settled = pending.dialog_closed_ms.is_none_or(|closed| {
            now_ms.saturating_sub(closed) >= SCENE_FOLLOW_AFTER_DIALOG_CLOSE_MS
        });
        let close_cap_applies = !self.host_in_scene;
        if pause_menu_open
            && (pending.pause_menu_close_requests < PAUSE_MENU_CLOSE_MAX_REQUESTS_PER_SCENE_START
                || !close_cap_applies)
            && (!pending.injected || !close_cap_applies)
            && last_close_settled
        {
            pending.pause_menu_close_requests += 1;
            pending.dialog_closed_ms = Some(now_ms);
            return FollowStep::ClosePauseMenu { waited_ms };
        }
        if pending.injected {
            return FollowStep::Wait;
        }
        if waited_ms < SCENE_FOLLOW_GRACE_MS && pending.pause_menu_close_requests == 0 {
            return FollowStep::Wait;
        }
        if dialog_open && pending.dialog_closed_ms.is_none() {
            pending.dialog_closed_ms = Some(now_ms);
            return FollowStep::CloseDialog { waited_ms };
        }
        let closing_settled = pending.dialog_closed_ms.is_none_or(|closed| {
            now_ms.saturating_sub(closed) >= SCENE_FOLLOW_AFTER_DIALOG_CLOSE_MS
        });
        if !closing_settled {
            return FollowStep::Wait;
        }
        if pending.callbacks.is_empty() {
            return FollowStep::Wait;
        }
        pending.injected = true;
        FollowStep::Inject {
            waited_ms,
            callbacks: pending.callbacks.clone(),
        }
    }
}
