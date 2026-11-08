use std::sync::{Mutex, PoisonError};

use rc_game::Replicant;
use rc_game::stay_together::FollowRole;
use rc_game::view_follow::{
    DoorView, SceneWatch, ViewAnnouncer, ViewFollow, decide_view_follow, door_chain_event,
    view_chain,
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

pub fn note_door_callback(name: &str) {
    let Some(event) = door_chain_event(name) else {
        return;
    };
    *OWN_VIEW.lock().unwrap_or_else(PoisonError::into_inner) = Some(DoorView {
        chain: event.chain.to_owned(),
        inside: event.inside,
    });
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
        FollowRole::Host => send_own_view(net),
        FollowRole::Guest => follow_host_view(build, received),
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
            run_chain_part(build, &chain, true);
        }
        ViewFollow::Leave { chain } => {
            log_line(&format!(
                "view follow: the host is outside {chain}, running its exit here"
            ));
            run_chain_part(build, &chain, false);
        }
    }
}

fn run_chain_part(build: &VerifiedBuild, chain: &str, entry: bool) {
    let (Some(phase), Some(known)) = (local_phase(build), view_chain(chain)) else {
        return;
    };
    let names = if entry { known.entry } else { known.exit };
    for name in names {
        let call = StoryCall {
            phase,
            pack: 0,
            function: (*name).to_owned(),
            args: Vec::new(),
        };
        let queued = queue_call(build, &call).is_some();
        log_line(&format!("view follow: {name} queued ({queued})"));
    }
}
