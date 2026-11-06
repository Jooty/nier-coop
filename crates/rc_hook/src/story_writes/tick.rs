use rc_game::stay_together::FollowRole;
use std::sync::{Mutex, PoisonError};

use rc_game::marker::read_world_flags;
use rc_game::story_writes::{HostChangeLog, host_write_outcome};
use rc_proto::{Message, StoryChange};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, NetState, net_clock_ms};
use crate::story_progress::{apply_change, describe_changes, read_live_arrays};
use crate::story_writes::calls::{send_talker_writes, talk_callback_running};

const MAX_WAITING_WRITE_MESSAGES: usize = 16;

static HOST_CHANGES: Mutex<HostChangeLog> = Mutex::new(HostChangeLog::new());
static WAITING_WRITES: Mutex<Vec<Message>> = Mutex::new(Vec::new());

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let arrived = net.take_relayed(|message| matches!(message, Message::StoryWrite(_)));
    if view.state != NetState::Connected {
        HOST_CHANGES
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        WAITING_WRITES
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        return;
    }
    match FollowRole::from_slot(view.local_slot) {
        FollowRole::Guest => {
            let window_open = read_world_flags(&InProcessMemory::new(build))
                .is_ok_and(|flags| flags.talk_window_open);
            send_talker_writes(build, talk_callback_running() && window_open);
        }
        FollowRole::Host => apply_guest_writes(build, arrived),
        FollowRole::Unknown => {}
    }
}

fn apply_guest_writes(build: &VerifiedBuild, arrived: Vec<Message>) {
    let mut waiting = WAITING_WRITES
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    waiting.extend(arrived);
    if waiting.len() > MAX_WAITING_WRITE_MESSAGES {
        let surplus = waiting.len() - MAX_WAITING_WRITE_MESSAGES;
        waiting.drain(..surplus);
        log_line(&format!(
            "talk-write: {surplus} waiting guest writes dropped, too many"
        ));
    }
    let mut host_changes = HOST_CHANGES.lock().unwrap_or_else(PoisonError::into_inner);
    while let Some(Message::StoryWrite(write)) = waiting.first().cloned() {
        let Some(live) = read_live_arrays(build) else {
            log_line(
                "talk-write: host could not read its story state, writes kept for the next tick",
            );
            return;
        };
        let outcome = host_write_outcome(&live, &write.changes, &host_changes, net_clock_ms());
        for &change in &outcome.apply {
            apply_change(build, change);
            host_changes.note_applied_from_guest(change, net_clock_ms());
        }
        log_line(&format!(
            "talk-write: host applied {} changes, {} already held, {} ignored (host changed them in the last 3 s): {}",
            outcome.apply.len(),
            outcome.already_held,
            outcome.ignored_recent_host_change,
            describe_changes(&outcome.apply)
        ));
        waiting.remove(0);
    }
    waiting.clear();
}

pub fn note_host_story_changes(changes: &[StoryChange]) {
    HOST_CHANGES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_feed_changes(changes, net_clock_ms());
}
