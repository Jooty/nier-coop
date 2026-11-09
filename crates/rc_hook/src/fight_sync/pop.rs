use std::panic;
use std::sync::PoisonError;

use rc_game::cutscene_skip::TRIGGER_MANAGER_RVA;
use rc_game::fight_sync::{
    CALLBACK_REGISTRY_ENTRY_COUNT, GuestPop, HostStreamOut, callback_registry_kind_address,
    is_registered_callback_kind, pop_source,
};
use rc_game::stay_together::FollowRole;
use rc_game::{GameMemory, PhaseEntry};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::fight_sync::state::{
    BUILD, GUEST, HOST, IN_FLIGHT_ARGUMENTS, current_phase_label, here,
};
use crate::memory::InProcessMemory;
use crate::net;
use crate::story_relay::{is_player_local_message_handler, read_arguments};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FightPop {
    Outside,
    Hold,
    Replayed,
    HostWindow,
}

pub(crate) fn before_pop(manager: u64) {
    let _ = panic::catch_unwind(|| {
        let Some(build) = BUILD.get() else {
            return;
        };
        if !HOST
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_armed()
        {
            return;
        }
        let count = registered_callback_count(build, manager);
        let outs = HOST
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .registered_callbacks(count);
        send_host_outs(build, outs);
    });
}

pub(crate) fn judge_pop(manager: u64, name: &str, arguments: u64) -> FightPop {
    panic::catch_unwind(|| judge(manager, name, arguments)).unwrap_or(FightPop::Outside)
}

pub(crate) fn pop_ran(name: &str, arguments: u64) {
    let _ = panic::catch_unwind(|| {
        let Some(build) = BUILD.get() else {
            return;
        };
        let Some(here) = here(build).filter(|here| here.role == FollowRole::Host) else {
            return;
        };
        let Some(phase) = here.local_phase else {
            return;
        };
        let mut outs = sync_host_phase(build);
        let (in_window, more_local_ids) = {
            let host = HOST.lock().unwrap_or_else(PoisonError::into_inner);
            let window = host.open_window();
            (
                window.is_some(),
                window.map_or(&[][..], |window| window.player_local_messages),
            )
        };
        let args = if in_window {
            read_arguments(&InProcessMemory::new(build), arguments).unwrap_or_else(|| {
                log_line(&format!(
                    "fight stream: {name}'s arguments could not be read, streamed without them"
                ));
                Vec::new()
            })
        } else {
            Vec::new()
        };
        let registered_local = is_player_local_message_handler(phase, name, more_local_ids);
        outs.extend(HOST.lock().unwrap_or_else(PoisonError::into_inner).popped(
            name,
            args,
            registered_local,
        ));
        send_host_outs(build, outs);
    });
}

pub(crate) fn function_traced(name: &str) {
    host_step(|host| host.function_ran(name));
}

pub(crate) fn note_registration() {
    host_step(rc_game::fight_sync::HostFightStream::callback_registered);
}

pub(super) fn sync_host_phase(build: &VerifiedBuild) -> Vec<HostStreamOut> {
    let Some(label) = current_phase_label(build) else {
        return Vec::new();
    };
    let Some(phase) = PhaseEntry::index_of(&label).and_then(|index| u16::try_from(index).ok())
    else {
        return Vec::new();
    };
    HOST.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .enter_phase(phase, &label)
}

pub(super) fn registered_callback_count(build: &VerifiedBuild, manager: u64) -> usize {
    let memory = InProcessMemory::new(build);
    (0..CALLBACK_REGISTRY_ENTRY_COUNT)
        .filter(|index| {
            memory
                .read_i32(callback_registry_kind_address(manager, *index))
                .is_ok_and(is_registered_callback_kind)
        })
        .count()
}

pub(super) fn trigger_manager(build: &VerifiedBuild) -> u64 {
    build.module_base() + u64::from(TRIGGER_MANAGER_RVA)
}

pub(super) fn send_host_outs(build: &VerifiedBuild, outs: Vec<HostStreamOut>) {
    let net = net::handle();
    for out in outs {
        match &out {
            HostStreamOut::Armed(window) => {
                log_line(&format!(
                    "fight stream: fight {}-{} armed by its start id, it opens on the next callback registration",
                    window.number, window.part
                ));
                let count = registered_callback_count(build, trigger_manager(build));
                let more = HOST
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .registered_callbacks(count);
                send_host_outs(build, more);
            }
            HostStreamOut::Opened {
                phase,
                epoch,
                window,
            } => log_line(&format!(
                "fight stream: fight {}-{} opened in phase {phase} (epoch {epoch}), every pop is sent to the guest",
                window.number, window.part
            )),
            HostStreamOut::Entry(entry) => log_line(&format!(
                "fight stream: {} (seq {}) runs here, sent",
                entry.name, entry.seq
            )),
            HostStreamOut::Closed {
                window,
                cause,
                end_seq,
                ..
            } => log_line(&format!(
                "fight stream: fight {}-{} closed ({cause:?}) after {end_seq} entries",
                window.number, window.part
            )),
        }
        if let (Some(net), Some(message)) = (net, out.message()) {
            net.send_relayed(message);
        }
    }
}

fn host_step(step: impl FnOnce(&mut rc_game::fight_sync::HostFightStream) -> Vec<HostStreamOut>) {
    let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let Some(build) = BUILD.get() else {
            return;
        };
        if here(build).is_none_or(|here| here.role != FollowRole::Host) {
            return;
        }
        let mut outs = sync_host_phase(build);
        outs.extend(step(
            &mut HOST.lock().unwrap_or_else(PoisonError::into_inner),
        ));
        send_host_outs(build, outs);
    }));
}

fn judge(manager: u64, name: &str, arguments: u64) -> FightPop {
    let Some(build) = BUILD.get() else {
        return FightPop::Outside;
    };
    let Some(here) = here(build) else {
        return FightPop::Outside;
    };
    let Some(phase) = here.local_phase else {
        return FightPop::Outside;
    };
    match here.role {
        FollowRole::Host => {
            let open = HOST
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_open_in(phase);
            if open {
                FightPop::HostWindow
            } else {
                FightPop::Outside
            }
        }
        FollowRole::Guest => judge_guest_pop(manager, name, arguments, phase),
        FollowRole::Unknown => FightPop::Outside,
    }
}

fn judge_guest_pop(manager: u64, name: &str, arguments: u64, phase: u16) -> FightPop {
    let Some(source) = pop_source(manager, arguments) else {
        return FightPop::Outside;
    };
    let more_local_ids = GUEST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .window()
        .map_or(&[][..], |window| window.player_local_messages);
    let registered_local = is_player_local_message_handler(phase, name, more_local_ids);
    let verdict = GUEST.lock().unwrap_or_else(PoisonError::into_inner).pop(
        name,
        source,
        Some(phase),
        registered_local,
    );
    match verdict {
        GuestPop::RunReplayed { seq } => {
            *IN_FLIGHT_ARGUMENTS
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = None;
            log_line(&format!(
                "fight stream: the host's {name} (seq {seq}) runs here"
            ));
            FightPop::Replayed
        }
        GuestPop::RunPlayerLocal => {
            log_line(&format!(
                "fight stream: {name} is player-local, your game runs its own"
            ));
            FightPop::Outside
        }
        GuestPop::RunPartnerCall | GuestPop::Outside => FightPop::Outside,
        GuestPop::Hold => {
            log_line(&format!(
                "fight stream: your game's {name} not run, the host's stream runs the fight"
            ));
            FightPop::Hold
        }
    }
}
