use std::panic;
use std::sync::{Mutex, PoisonError};

use rc_game::callback_gate::{
    CALLBACK_GATE_HOST_GRACE_MS, CallbackGate, GateRole, GateStep, LocalFire, PartnerFire,
    is_gated_callback,
};
use rc_game::stay_together::FollowRole;
use rc_proto::{CallbackFired, Message, StoryCall};

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::scene_follow::{local_phase, note_injected};
use crate::diagnostics::log_line;
use crate::net::{self, NetShared, NetState, NetThread, net_clock_ms};
use crate::stay_together::{close_main_menu, main_menu_open};
use crate::story_relay::inject::queue_call;

static GATE: Mutex<CallbackGate> = Mutex::new(CallbackGate::new());

pub fn allow_next(name: &str) {
    GATE.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .allow_next(name, net_clock_ms());
}

pub fn hold_fired_callback(name: &str) -> bool {
    if !is_gated_callback(name) {
        return false;
    }
    panic::catch_unwind(|| decide(name)).unwrap_or(false)
}

fn decide(name: &str) -> bool {
    let Some(net) = net::handle() else {
        return false;
    };
    let view = net.view();
    let Some(role) = gate_role(&view) else {
        return false;
    };
    let Some(phase) = shared_phase(&view) else {
        return false;
    };
    let verdict = GATE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .local_fire(role, name, net_clock_ms());
    let announce = || {
        net.send_relayed(Message::CallbackFired(CallbackFired {
            phase,
            name: name.to_owned(),
        }));
    };
    match (role, verdict) {
        (GateRole::Host, LocalFire::RunReleased) => {
            log_line(&format!(
                "callback follow: {name} starts from the guest's step, sent to the guest"
            ));
            announce();
        }
        (GateRole::Host, _) => {
            log_line(&format!(
                "callback follow: you fired {name}, sent to the guest"
            ));
            announce();
        }
        (GateRole::Guest, LocalFire::Hold) => {
            log_line(&format!(
                "callback follow: your game fired {name} before the host did, held; the host is asked to fire it"
            ));
            announce();
            return true;
        }
        (GateRole::Guest, LocalFire::RunReleased) => {
            log_line(&format!("callback follow: {name} released, it runs here"));
        }
        (GateRole::Guest, _) => {
            log_line(&format!(
                "callback follow: the host fired {name} first, it runs here"
            ));
        }
    }
    false
}

pub(super) fn tick(build: &VerifiedBuild, net: &NetThread, view: &NetShared) {
    let received = net.take_relayed(|message| matches!(message, Message::CallbackFired(_)));
    let role = gate_role(view);
    let (Some(role), Some(phase)) = (role, shared_phase(view)) else {
        let now = net_clock_ms();
        let mut gate = GATE.lock().unwrap_or_else(PoisonError::into_inner);
        let released = gate.release_held(now);
        if view.state != NetState::Connected {
            gate.reset();
        }
        drop(gate);
        let phase = local_phase(build);
        for name in released {
            log_line(&format!(
                "callback follow: the host is no longer in the same play, held {name} runs here"
            ));
            if let Some(phase) = phase {
                start(build, phase, &name, now);
            }
        }
        return;
    };
    let local = local_phase(build);
    let now = net_clock_ms();
    let mut gate = GATE.lock().unwrap_or_else(PoisonError::into_inner);
    let mut releases = Vec::new();
    for message in received {
        let Message::CallbackFired(fired) = message else {
            continue;
        };
        if fired.phase != phase || Some(fired.phase) != local {
            log_line(&format!(
                "callback follow: {} fired in another phase on the partner's game, ignored",
                fired.name
            ));
            continue;
        }
        match gate.partner_fire(role, &fired.name, now) {
            PartnerFire::Release { waited_ms } => {
                releases.push(fired.name.clone());
                log_line(&format!(
                    "callback follow: the host fired {}, your held copy starts now (held {waited_ms} ms)",
                    fired.name
                ));
            }
            PartnerFire::Noted => log_line(&format!(
                "callback follow: the partner fired {}, your game has not yet",
                fired.name
            )),
            PartnerFire::HostWaits => log_line(&format!(
                "callback follow: the guest fired {} and your game has not, waiting {CALLBACK_GATE_HOST_GRACE_MS} ms for it",
                fired.name
            )),
            PartnerFire::HostFiredFirst => log_line(&format!(
                "callback follow: the guest fired {}, your game had fired it already",
                fired.name
            )),
        }
    }
    let steps = gate.step(role, main_menu_open(build), now);
    drop(gate);
    for name in releases {
        start(build, phase, &name, now);
    }
    for step in steps {
        match step {
            GateStep::GaveUp { name, waited_ms } => {
                log_line(&format!(
                    "callback follow: gave up waiting for the host to fire {name} after {waited_ms} ms, it runs here"
                ));
                start(build, phase, &name, now);
            }
            GateStep::CloseDialog { name, waited_ms } => {
                log_line(&format!(
                    "callback follow: your game still has not fired {name} after {waited_ms} ms with a prompt open, closing it"
                ));
                close_main_menu(build, "to follow the guest's step");
            }
            GateStep::Inject { name, waited_ms } => {
                log_line(&format!(
                    "callback follow: your game never fired {name} ({waited_ms} ms), starting it from the guest's step"
                ));
                start(build, phase, &name, now);
            }
        }
    }
}

fn start(build: &VerifiedBuild, phase: u16, name: &str, now: u64) {
    note_injected(name, now);
    let call = StoryCall {
        phase,
        pack: 0,
        function: name.to_owned(),
        args: Vec::new(),
    };
    let queued = queue_call(build, &call).is_some();
    log_line(&format!("callback follow: {name} queued ({queued})"));
}

fn gate_role(view: &NetShared) -> Option<GateRole> {
    if !view.partner_in_play() {
        return None;
    }
    match FollowRole::from_slot(view.local_slot) {
        FollowRole::Host => Some(GateRole::Host),
        FollowRole::Guest => Some(GateRole::Guest),
        FollowRole::Unknown => None,
    }
}

fn shared_phase(view: &NetShared) -> Option<u16> {
    let local = view.local.as_ref()?.phase;
    let remote = view.remote.as_ref()?.state.phase;
    (local == remote).then_some(local)
}
