use crate::load_gate::StoryPlace;
use crate::story_steps::step_lines::{
    watch_host_not_reached_line, watch_own_after_line, watch_own_before_line,
    watch_own_without_host_line,
};
use crate::story_steps::{
    MOST_WATCHED_STEPS_KEPT, ObservedDone, STORY_STEP_GIVE_UP_MS, StoryStepDone,
    WATCHED_STEP_COMPARE_AFTER_MS,
};

#[derive(Debug, Clone, Copy)]
struct OwnDoneSeen {
    id: u32,
    class: &'static str,
    at_ms: u64,
    reported: bool,
}

#[derive(Debug, Clone, Copy)]
struct HostDoneHeard {
    id: u32,
    at_ms: u64,
    reported: bool,
}

/// The guest's side in mode `watch`: compares its own dones with the host's, holds nothing.
#[derive(Debug, Clone, Default)]
pub(crate) struct GuestWatch {
    own: Vec<OwnDoneSeen>,
    host: Vec<HostDoneHeard>,
}

impl GuestWatch {
    pub(crate) fn own_done(&mut self, done: &ObservedDone, now_ms: u64, logs: &mut Vec<String>) {
        if let Some(index) = self.host.iter().position(|host| host.id == done.id) {
            let host = self.host.remove(index);
            logs.push(watch_own_after_line(
                done.id,
                done.class_name(),
                now_ms.saturating_sub(host.at_ms),
            ));
            return;
        }
        self.own.retain(|own| own.id != done.id);
        if self.own.len() >= MOST_WATCHED_STEPS_KEPT {
            self.own.remove(0);
        }
        self.own.push(OwnDoneSeen {
            id: done.id,
            class: done.class_name(),
            at_ms: now_ms,
            reported: false,
        });
    }

    pub(crate) fn hear_done(&mut self, done: &StoryStepDone, now_ms: u64, logs: &mut Vec<String>) {
        if let Some(index) = self.own.iter().position(|own| own.id == done.id) {
            let own = self.own.remove(index);
            logs.push(watch_own_before_line(
                own.id,
                own.class,
                now_ms.saturating_sub(own.at_ms),
            ));
            return;
        }
        self.host.retain(|host| host.id != done.id);
        if self.host.len() >= MOST_WATCHED_STEPS_KEPT {
            self.host.remove(0);
        }
        self.host.push(HostDoneHeard {
            id: done.id,
            at_ms: now_ms,
            reported: false,
        });
    }

    pub(crate) fn step(&mut self, place: &StoryPlace, now_ms: u64, logs: &mut Vec<String>) {
        for host in &mut self.host {
            if !host.reported && now_ms.saturating_sub(host.at_ms) >= WATCHED_STEP_COMPARE_AFTER_MS
            {
                host.reported = true;
                logs.push(watch_host_not_reached_line(host.id, place));
            }
        }
        for own in &mut self.own {
            if !own.reported && now_ms.saturating_sub(own.at_ms) >= STORY_STEP_GIVE_UP_MS {
                own.reported = true;
                logs.push(watch_own_without_host_line(own.id, own.class));
            }
        }
    }

    pub(crate) fn forget(&mut self, id: u32) -> bool {
        let before = self.own.len() + self.host.len();
        self.own.retain(|own| own.id != id);
        self.host.retain(|host| host.id != id);
        self.own.len() + self.host.len() != before
    }

    pub(crate) fn has_lists(&self) -> bool {
        !self.own.is_empty() || !self.host.is_empty()
    }

    pub(crate) fn clear_lists(&mut self) {
        *self = Self::default();
    }
}
