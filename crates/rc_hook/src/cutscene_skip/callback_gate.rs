use std::panic;
use std::sync::{Mutex, PoisonError};

use rc_game::GameMemory;
use rc_game::callback_gate::{
    CALLBACK_GATE_HOST_GRACE_MS, CallbackGate, EndArrival, EndFollow, GateRole, GateStep,
    LocalFire, MaskIdleTimer, PartnerFire, PartnerStartedCopies, PendingEnd, RanEnds,
    TALK_START_AFTER_COMMAND_MS, WaitStep, end_follow, is_end_callback, is_gated_callback,
    is_scene_talk, wait_step,
};
use rc_game::marker::{CONTROL_MASK_FIRST_RVA, CONTROL_MASK_SLOT_COUNT, read_world_flags};
use rc_game::stay_together::FollowRole;
use rc_game::story_calls::is_event_callback_name;
use rc_proto::{CallbackFired, Message, StoryCall};

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::scene_follow::{local_phase, note_injected};
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, NetShared, NetState, NetThread, net_clock_ms};
use crate::stay_together::{close_main_menu, close_pause_menu, main_menu_open, pause_menu_open};
use crate::story_relay::inject::queue_call;
use crate::story_writes::begin_end_watch;

static GATE: Mutex<CallbackGate> = Mutex::new(CallbackGate::new());
static WAITING_ENDS: Mutex<Vec<PendingEnd>> = Mutex::new(Vec::new());
static STARTED_FROM_PARTNER: Mutex<PartnerStartedCopies> = Mutex::new(PartnerStartedCopies::new());
static MASK_IDLE: Mutex<MaskIdleTimer> = Mutex::new(MaskIdleTimer::new());
static RAN_ENDS: Mutex<RanEnds> = Mutex::new(RanEnds::new());
static LATEST_PARTNER_END: Mutex<Option<String>> = Mutex::new(None);
static LAST_TALK_COMMAND_MS: Mutex<Option<u64>> = Mutex::new(None);

pub fn note_talk_command() {
    *LAST_TALK_COMMAND_MS
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(net_clock_ms());
}

fn take_recent_talk_command() -> bool {
    let noted = LAST_TALK_COMMAND_MS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    noted.is_some_and(|at| net_clock_ms().saturating_sub(at) <= TALK_START_AFTER_COMMAND_MS)
}

pub fn latest_partner_end() -> Option<(String, bool)> {
    let name = LATEST_PARTNER_END
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()?;
    let ran_here = end_ran_here(&name);
    Some((name, ran_here))
}

pub fn end_ran_here(name: &str) -> bool {
    RAN_ENDS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .contains_name(name)
}

pub fn allow_next(name: &str) {
    GATE.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .allow_next(name, net_clock_ms());
}

pub fn hold_fired_callback(name: &str) -> bool {
    if is_end_callback(name) {
        let _ = panic::catch_unwind(|| announce_end(name));
        return false;
    }
    if !is_gated_callback(name) {
        return false;
    }
    let after_talk_command = is_event_callback_name(name) && take_recent_talk_command();
    if after_talk_command || is_scene_talk(name) {
        return decide_talk(name, after_talk_command);
    }
    panic::catch_unwind(|| decide(name)).unwrap_or(false)
}

fn decide_talk(name: &str, after_talk_command: bool) -> bool {
    if after_talk_command {
        log_line(&format!("talk owner: {name} is yours, runs here"));
    } else {
        log_line(&format!(
            "talk owner: {name} is a scene talk, runs here only"
        ));
    }
    false
}

fn announce_end(name: &str) {
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let Some(role) = gate_role(&view) else {
        return;
    };
    let Some(phase) = shared_phase(&view) else {
        return;
    };
    note_ran_end(name, phase);
    let started_from_partner = STARTED_FROM_PARTNER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take(name, net_clock_ms());
    if started_from_partner {
        return;
    }
    let receiver = if role == GateRole::Guest {
        "host"
    } else {
        "guest"
    };
    log_line(&format!(
        "callback follow: your game ended {name}, sent to the {receiver}"
    ));
    net.send_relayed(Message::CallbackFired(CallbackFired {
        phase,
        name: name.to_owned(),
    }));
    if role == GateRole::Guest {
        begin_end_watch();
    }
}

fn note_ran_end(name: &str, phase: u16) {
    RAN_ENDS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note(name, phase);
}

fn partner_label(role: GateRole) -> &'static str {
    match role {
        GateRole::Host => "guest",
        GateRole::Guest => "host",
    }
}

fn partner_end_verdict(
    build: &VerifiedBuild,
    view: &NetShared,
    name: &str,
    phase: u16,
) -> EndFollow {
    let arrival = EndArrival {
        message_phase: phase,
        shared_phase: shared_phase(view),
        local_phase: local_phase(build),
        menu_open: main_menu_open(build) || pause_menu_open(build),
        local_in_own_talk: read_world_flags(&InProcessMemory::new(build))
            .is_ok_and(|flags| flags.talk_window_open),
        control_mask_settled: control_mask_settled(build),
        already_ran_here: RAN_ENDS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(name, phase),
    };
    end_follow(&arrival)
}

fn control_mask_settled(build: &VerifiedBuild) -> bool {
    let memory = InProcessMemory::new(build);
    let base = memory.module_base();
    let all_slots_zero = (0..CONTROL_MASK_SLOT_COUNT).all(|slot| {
        memory
            .read_i32(base + u64::from(CONTROL_MASK_FIRST_RVA) + 4 * u64::from(slot))
            .is_ok_and(|mask| mask == 0)
    });
    MASK_IDLE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .observe(all_slots_zero, net_clock_ms())
}

fn follow_end(build: &VerifiedBuild, role: GateRole, view: &NetShared, fired: &CallbackFired) {
    let now = net_clock_ms();
    let partner = partner_label(role);
    let mirrored_copy = role == GateRole::Guest
        && STARTED_FROM_PARTNER
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(&fired.name, now)
        && shared_phase(view) == Some(fired.phase)
        && local_phase(build) == Some(fired.phase);
    if mirrored_copy {
        log_line(&format!(
            "callback follow: the host ended {}, your copy of it ends now",
            fired.name
        ));
        start(build, fired.phase, &fired.name, now);
        return;
    }
    match partner_end_verdict(build, view, &fired.name, fired.phase) {
        EndFollow::Mirror => {
            log_line(&format!(
                "callback follow: the {partner} ended {}, your game runs it now",
                fired.name
            ));
            start(build, fired.phase, &fired.name, now);
        }
        EndFollow::Drop(reason) if reason.can_wait() => {
            log_line(&format!(
                "callback follow: the {partner} ended {}, waits for {}",
                fired.name,
                reason.reason()
            ));
            let mut waiting = WAITING_ENDS.lock().unwrap_or_else(PoisonError::into_inner);
            waiting.retain(|pending| pending.name != fired.name);
            waiting.push(PendingEnd {
                name: fired.name.clone(),
                phase: fired.phase,
                since_ms: now,
            });
        }
        EndFollow::Drop(reason) => log_line(&format!(
            "callback follow: the {partner} ended {}, not run here: {}",
            fired.name,
            reason.reason()
        )),
    }
}

fn run_waiting_ends(build: &VerifiedBuild, role: GateRole, view: &NetShared) {
    let partner = partner_label(role);
    let pending: Vec<PendingEnd> =
        std::mem::take(&mut *WAITING_ENDS.lock().unwrap_or_else(PoisonError::into_inner));
    if pending.is_empty() {
        return;
    }
    let now = net_clock_ms();
    let mut still_waiting = Vec::new();
    for end in pending {
        let verdict = partner_end_verdict(build, view, &end.name, end.phase);
        match wait_step(end.since_ms, now, verdict) {
            WaitStep::Run { waited_ms } => {
                log_line(&format!(
                    "callback follow: the {partner} ended {}, runs now after waiting {}s",
                    end.name,
                    waited_ms / 1000
                ));
                start(build, end.phase, &end.name, now);
            }
            WaitStep::KeepWaiting => still_waiting.push(end),
            WaitStep::GiveUp { waited_ms } => log_line(&format!(
                "callback follow: the {partner} ended {}, dropped after waiting {}s",
                end.name,
                waited_ms / 1000
            )),
            WaitStep::Drop(reason) => log_line(&format!(
                "callback follow: the {partner} ended {}, not run here after waiting: {}",
                end.name,
                reason.reason()
            )),
        }
    }
    WAITING_ENDS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .extend(still_waiting);
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
    let partner_place = view.partner_place();
    let verdict = {
        let mut gate = GATE.lock().unwrap_or_else(PoisonError::into_inner);
        gate.note_partner_here(partner_place.is_same());
        gate.local_fire(role, name, net_clock_ms())
    };
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
        (GateRole::Guest, LocalFire::RunAlone) => {
            log_line(&format!(
                "callback follow: your game fired {name} with the host {}, it runs here only",
                partner_place.describe()
            ));
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
    let (ends, received): (Vec<Message>, Vec<Message>) = net
        .take_relayed(|message| matches!(message, Message::CallbackFired(_)))
        .into_iter()
        .partition(|message| matches!(message, Message::CallbackFired(fired) if is_end_callback(&fired.name)));
    let role = gate_role(view);
    let (Some(role), Some(phase)) = (role, shared_phase(view)) else {
        let now = net_clock_ms();
        let mut gate = GATE.lock().unwrap_or_else(PoisonError::into_inner);
        let released = gate.release_held(now);
        if view.state != NetState::Connected {
            gate.reset();
            WAITING_ENDS
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clear();
            STARTED_FROM_PARTNER
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .reset();
            RAN_ENDS
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .reset();
            *LATEST_PARTNER_END
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = None;
        }
        drop(gate);
        for message in ends {
            if let Message::CallbackFired(fired) = message {
                log_line(&format!(
                    "callback follow: the partner ended {}, not run here: you are not in the same play",
                    fired.name
                ));
            }
        }
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
    for message in ends {
        if let Message::CallbackFired(fired) = message {
            *LATEST_PARTNER_END
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Some(fired.name.clone());
            follow_end(build, role, view, &fired);
        }
    }
    run_waiting_ends(build, role, view);
    let local = local_phase(build);
    let now = net_clock_ms();
    let partner_place = view.partner_place();
    let mut gate = GATE.lock().unwrap_or_else(PoisonError::into_inner);
    gate.note_partner_here(partner_place.is_same());
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
            PartnerFire::FollowHost => {
                releases.push(fired.name.clone());
                log_line(&format!(
                    "callback follow: the host started {} and you are in the same place, it starts here too",
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
            PartnerFire::Elsewhere => log_line(&format!(
                "callback follow: the guest fired {} {}, not followed here",
                fired.name,
                partner_place.describe()
            )),
            PartnerFire::HostFiredFirst => log_line(&format!(
                "callback follow: the guest fired {}, your game had fired it already",
                fired.name
            )),
        }
    }
    let steps = gate.step(role, main_menu_open(build), pause_menu_open(build), now);
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
            GateStep::ClosePauseMenu { name, waited_ms } => {
                log_line(&format!(
                    "callback follow: your pause menu is open {waited_ms} ms after the guest fired {name}, closing it"
                ));
                close_pause_menu(build, "to follow the guest's step");
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
    if is_end_callback(name) {
        crate::pickups::open_injected_end_window(name);
        note_ran_end(name, phase);
    }
    note_injected(name, now);
    STARTED_FROM_PARTNER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note(name, now);
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
