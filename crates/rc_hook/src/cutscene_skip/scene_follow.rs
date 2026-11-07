use std::sync::{Mutex, PoisonError};

use rc_game::scene_follow::{FollowStep, InjectedCallbacks, RecentCallbacks, SceneFollowWatch};
use rc_game::stay_together::FollowRole;
use rc_game::{PhaseEntry, Replicant};
use rc_proto::{Message, SceneFollow, StoryCall};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{NetShared, NetState, NetThread, net_clock_ms};
use crate::stay_together::{close_main_menu, main_menu_open};
use crate::story_relay::inject::queue_call;

static RECENT: Mutex<RecentCallbacks> = Mutex::new(RecentCallbacks::new());
static WATCH: Mutex<SceneFollowWatch> = Mutex::new(SceneFollowWatch::new());
static INJECTED: Mutex<InjectedCallbacks> = Mutex::new(InjectedCallbacks::new());
static HOST_WAS_IN_SCENE: Mutex<bool> = Mutex::new(false);

pub fn note_fired_callback(name: &str) {
    RECENT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note(name, net_clock_ms());
}

pub(super) fn note_injected(name: &str, now_ms: u64) {
    INJECTED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .add(name, now_ms);
}

pub fn is_injected_duplicate(name: &str) -> bool {
    let duplicate = INJECTED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_duplicate(name, net_clock_ms());
    if duplicate {
        log_line(&format!(
            "scene follow: dropped your own fire of {name}, the host's copy of it already ran"
        ));
    }
    duplicate
}

pub(super) fn tick(build: &VerifiedBuild, net: &NetThread, view: &NetShared, local_in_scene: bool) {
    let received = net.take_relayed(|message| matches!(message, Message::SceneFollow(_)));
    if view.state != NetState::Connected {
        WATCH.lock().unwrap_or_else(PoisonError::into_inner).reset();
        *HOST_WAS_IN_SCENE
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = false;
        return;
    }
    match FollowRole::from_slot(view.local_slot) {
        FollowRole::Host => host_tick(build, net, view, local_in_scene),
        FollowRole::Guest => guest_tick(build, local_in_scene, received),
        FollowRole::Unknown => {}
    }
}

fn host_tick(build: &VerifiedBuild, net: &NetThread, view: &NetShared, in_scene: bool) {
    let mut was = HOST_WAS_IN_SCENE
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let entered = in_scene && !*was;
    *was = in_scene;
    drop(was);
    if !entered || !view.partner_in_play() {
        return;
    }
    let names = RECENT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .names_before_scene(net_clock_ms());
    let Some(phase) = local_phase(build) else {
        return;
    };
    if names.is_empty() {
        log_line(
            "scene follow: you entered a scene, but no event callback fired just before it, nothing sent to the guest",
        );
        return;
    }
    log_line(&format!(
        "scene follow: you entered a scene after {names:?}, sent to the guest"
    ));
    net.send_relayed(Message::SceneFollow(SceneFollow {
        phase,
        callbacks: names,
    }));
}

fn guest_tick(build: &VerifiedBuild, local_in_scene: bool, received: Vec<Message>) {
    let now = net_clock_ms();
    let mut watch = WATCH.lock().unwrap_or_else(PoisonError::into_inner);
    let local = local_phase(build);
    for message in received {
        let Message::SceneFollow(follow) = message else {
            continue;
        };
        if Some(follow.phase) != local {
            log_line("scene follow: the host's scene is in another phase, ignored");
            continue;
        }
        log_line(&format!(
            "scene follow: the host entered a scene after {:?}, your game gets {} ms to enter it",
            follow.callbacks,
            rc_game::scene_follow::SCENE_FOLLOW_GRACE_MS
        ));
        watch.receive(follow.callbacks, now);
    }
    let dialog_open = main_menu_open(build);
    let step = watch.step(local_in_scene, dialog_open, now);
    drop(watch);
    match step {
        FollowStep::Idle | FollowStep::Wait => {}
        FollowStep::Followed { waited_ms } => {
            log_line(&format!(
                "scene follow: you are in the host's scene ({waited_ms} ms after it began)"
            ));
        }
        FollowStep::CloseDialog { waited_ms } => {
            log_line(&format!(
                "scene follow: still out of the host's scene after {waited_ms} ms with a prompt open, closing it"
            ));
            close_main_menu(build, "to enter the host's scene");
        }
        FollowStep::Inject {
            waited_ms,
            callbacks,
        } => inject(build, local, waited_ms, &callbacks, now),
        FollowStep::GaveUp { waited_ms } => {
            log_line(&format!(
                "scene follow: gave up after {waited_ms} ms, your game never entered the host's scene"
            ));
        }
    }
}

fn inject(
    build: &VerifiedBuild,
    phase: Option<u16>,
    waited_ms: u64,
    callbacks: &[String],
    now: u64,
) {
    let Some(phase) = phase else {
        return;
    };
    for name in callbacks {
        let call = StoryCall {
            phase,
            pack: 0,
            function: name.clone(),
            args: Vec::new(),
        };
        note_injected(name, now);
        crate::cutscene_skip::callback_gate::allow_next(name);
        let queued = queue_call(build, &call).is_some();
        log_line(&format!(
            "scene follow: still out of the host's scene after {waited_ms} ms, starting {name} from the host's step (queued {queued})"
        ));
    }
}

pub(super) fn local_phase(build: &VerifiedBuild) -> Option<u16> {
    let game = Replicant::new(InProcessMemory::new(build));
    game.current_phase()
        .ok()
        .and_then(|name| PhaseEntry::index_of(&name))
        .and_then(|index| u16::try_from(index).ok())
}
