use std::sync::{Mutex, PoisonError};

use rc_game::Replicant;
use rc_game::stay_together::FollowRole;
use rc_game::view_follow::{
    DoorView, RetryAction, SceneWatch, ViewAnnouncer, ViewFollow, ViewRetry, decide_view_follow,
    view_after_callback, view_after_phase_check, view_chain,
};
use rc_proto::{Message, StoryCall, ViewState};

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::scene_follow::local_phase;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{NetShared, NetThread, net_clock_ms};
use crate::story_relay::inject::queue_call;

static OWN_VIEW: Mutex<Option<DoorView>> = Mutex::new(None);
static HOST_VIEW: Mutex<Option<DoorView>> = Mutex::new(None);
static ANNOUNCER: Mutex<ViewAnnouncer> = Mutex::new(ViewAnnouncer::new());
static WATCH: Mutex<SceneWatch> = Mutex::new(SceneWatch::new());
static RETRY: Mutex<ViewRetry> = Mutex::new(ViewRetry::new());
static OWN_VIEW_PHASE: Mutex<Option<u16>> = Mutex::new(None);

pub fn note_popped_callback(name: &str) {
    let ran = RETRY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_pop(name, net_clock_ms());
    if let Some(ran) = ran {
        log_line(&format!(
            "view follow: {} ran after {}s",
            ran.name,
            ran.after_ms / 1000
        ));
    }
    let mut own = OWN_VIEW.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(view) = view_after_callback(name, own.as_ref()) {
        *own = Some(view);
    }
}

pub fn note_fade_edge(started: bool) {
    RETRY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_fade(started);
}

pub fn note_scene_run() {
    WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .open(net_clock_ms());
}

pub(super) fn tick(build: &VerifiedBuild, net: &NetThread, view: &NetShared) {
    let received = net.take_relayed(|message| matches!(message, Message::ViewState(_)));
    if !view.partner_in_play() {
        forget_partner();
        return;
    }
    match FollowRole::from_slot(view.local_slot) {
        FollowRole::Host => {
            drop_view_on_zone_change(build);
            send_own_view(net);
        }
        FollowRole::Guest => {
            drop_view_on_zone_change(build);
            follow_host_view(build, received);
        }
        FollowRole::Unknown => {}
    }
}

fn forget_partner() {
    *HOST_VIEW.lock().unwrap_or_else(PoisonError::into_inner) = None;
    ANNOUNCER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .reset();
    WATCH.lock().unwrap_or_else(PoisonError::into_inner).reset();
    RETRY.lock().unwrap_or_else(PoisonError::into_inner).reset();
}

fn drop_view_on_zone_change(build: &VerifiedBuild) {
    let phase = local_phase(build);
    let mut last = OWN_VIEW_PHASE
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let mut own = OWN_VIEW.lock().unwrap_or_else(PoisonError::into_inner);
    *own = view_after_phase_check(own.take(), *last, phase);
    if phase.is_some() {
        *last = phase;
    }
}

fn drive_retry(build: &VerifiedBuild) {
    let actions = RETRY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .poll(net_clock_ms());
    let Some(phase) = local_phase(build) else {
        return;
    };
    for action in actions {
        match action {
            RetryAction::Queue(name) => {
                let call = StoryCall {
                    phase,
                    pack: 0,
                    function: name.clone(),
                    args: Vec::new(),
                };
                let queued = queue_call(build, &call).is_some();
                log_line(&format!("view follow: {name} queued ({queued})"));
            }
            RetryAction::GiveUp(name) => {
                log_line(&format!("view follow: {name} never ran"));
            }
        }
    }
}

fn send_own_view(net: &NetThread) {
    let own = OWN_VIEW
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let due = ANNOUNCER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_due(own.as_ref(), net_clock_ms());
    if let (true, Some(own)) = (due, own) {
        net.send_relayed(Message::ViewState(ViewState {
            chain: own.chain,
            inside: own.inside,
        }));
    }
}

fn follow_host_view(build: &VerifiedBuild, received: Vec<Message>) {
    for message in received {
        if let Message::ViewState(state) = message {
            *HOST_VIEW.lock().unwrap_or_else(PoisonError::into_inner) = Some(DoorView {
                chain: state.chain,
                inside: state.inside,
            });
        }
    }
    drive_retry(build);
    let Ok(position) = Replicant::new(InProcessMemory::new(build)).position() else {
        return;
    };
    let moved = WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .observe(net_clock_ms(), position);
    if !moved {
        return;
    }
    let host = HOST_VIEW
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let own = OWN_VIEW
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    match decide_view_follow(host.as_ref(), own.as_ref(), moved) {
        ViewFollow::Nothing => {}
        ViewFollow::Enter { chain } => {
            log_line(&format!(
                "view follow: the host is inside {chain}, running its entry here"
            ));
            run_chain_part(&chain, true);
        }
        ViewFollow::Leave { chain } => {
            log_line(&format!(
                "view follow: the host is outside {chain}, running its exit here"
            ));
            run_chain_part(&chain, false);
        }
    }
}

fn run_chain_part(chain: &str, entry: bool) {
    let Some(known) = view_chain(chain) else {
        return;
    };
    let names = if entry { known.entry } else { known.exit };
    RETRY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .request(names, net_clock_ms());
    log_line(&format!(
        "view follow: waiting for the scene to end before queueing {}",
        names.join(" and ")
    ));
}
