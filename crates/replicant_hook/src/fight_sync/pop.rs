use std::panic;
use std::sync::PoisonError;

use replicant_game::cutscene_skip::TRIGGER_MANAGER_RVA;
use replicant_game::fight_sync::{
    CALLBACK_REGISTRY_ENTRY_COUNT, GuestPop, HostPop, HostStreamOut, PopSource, WindowArming,
    callback_registry_kind_address, fired_registry_index, is_player_tested_callback_kind,
    is_pop_by_this_player, is_registered_callback_kind, pack_windows_line, pop_source,
};
use replicant_game::stay_together::FollowRole;
use replicant_game::{GameMemory, PhaseEntry};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::fight_sync::object_arms::armed_by_object;
use crate::fight_sync::state::{
    BUILD, GUEST, HOST, REGISTRY_KINDS_BEFORE_POP, current_phase_label, here,
};
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
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
        note_registry_kinds_for_the_guest(build, manager);
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
        let Some(here) = here(build) else {
            return;
        };
        if here.role == FollowRole::Guest {
            guest_ran_here(name, here.local_phase);
            return;
        }
        if here.role != FollowRole::Host {
            return;
        }
        let Some(phase) = here.local_phase else {
            return;
        };
        let mut outs = sync_host_phase(build);
        let (streamed_with_arguments, more_local_ids) = {
            let host = HOST.lock().unwrap_or_else(PoisonError::into_inner);
            let window = host.open_window();
            (
                window.is_some() || host.is_start_id(name),
                window.map_or(&[][..], |window| window.player_local_messages),
            )
        };
        let args = if streamed_with_arguments {
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
    let _ = panic::catch_unwind(|| {
        let Some(here) = BUILD.get().and_then(here) else {
            return;
        };
        if here.role == FollowRole::Guest {
            guest_ran_here(name, here.local_phase);
        }
    });
}

pub(crate) fn note_registration() {
    host_step(replicant_game::fight_sync::HostFightStream::callback_registered);
}

pub(super) fn sync_host_phase(build: &VerifiedBuild) -> Vec<HostStreamOut> {
    let Some(label) = current_phase_label(build) else {
        return Vec::new();
    };
    let Some(phase) = PhaseEntry::index_of(&label).and_then(|index| u16::try_from(index).ok())
    else {
        return Vec::new();
    };
    let mut host = HOST.lock().unwrap_or_else(PoisonError::into_inner);
    if host.pack() != label {
        log_line(&pack_windows_line(&label));
    }
    host.enter_phase(phase, &label)
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
            HostStreamOut::Armed {
                window,
                by,
                arming: WindowArming::ArmId,
            } => log_line(&format!(
                "fight window: {} armed by {by} (arm id), it opens now with {by} as entry 0",
                window.page
            )),
            HostStreamOut::Armed {
                window,
                by,
                arming: WindowArming::StartId,
            } => {
                log_line(&format!(
                    "fight window: {} armed by {by} (start id), it opens on the next callback registration",
                    window.page
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
                ..
            } => log_line(&format!(
                "fight window: {} opened in phase {phase} (epoch {epoch}), every pop is sent to the guest",
                window.page
            )),
            HostStreamOut::Restarted {
                phase,
                epoch,
                window,
            } => log_line(&format!(
                "fight stream: fight {}-{} restarted in phase {phase}, it goes on at epoch {epoch}",
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
                "fight window: {} closed ({cause:?}) after {end_seq} entries",
                window.page
            )),
            HostStreamOut::EntryEnded { .. } => {}
        }
        if let (Some(net), Some(message)) = (net, out.message()) {
            net.send_relayed(message);
        }
    }
}

fn guest_ran_here(name: &str, local_phase: Option<u16>) {
    GUEST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .ran_here(name, local_phase);
}

fn host_step(
    step: impl FnOnce(&mut replicant_game::fight_sync::HostFightStream) -> Vec<HostStreamOut>,
) {
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
    // Every queued pop leaves the object-armed list, whoever judges it.
    let queued_by_object = pop_source(manager, arguments) == Some(PopSource::Queued)
        && armed_by_object(arguments, name);
    let Some(here) = here(build) else {
        return FightPop::Outside;
    };
    let Some(phase) = here.local_phase else {
        return FightPop::Outside;
    };
    match here.role {
        FollowRole::Host => judge_host_pop(manager, name, arguments, phase),
        FollowRole::Guest => judge_guest_pop(manager, name, arguments, phase, queued_by_object),
        FollowRole::Unknown => FightPop::Outside,
    }
}

fn judge_host_pop(manager: u64, name: &str, arguments: u64, phase: u16) -> FightPop {
    let mut host = HOST.lock().unwrap_or_else(PoisonError::into_inner);
    let source = pop_source(manager, arguments).unwrap_or(PopSource::Fired);
    let open = host.is_open_in(phase);
    if source == PopSource::Queued && host.earlier_request_popped(name, arguments) {
        drop(host);
        log_line(&format!(
            "fight request: the guest's earlier {name} popped here after its fight moved on, it runs"
        ));
        return if open {
            FightPop::HostWindow
        } else {
            FightPop::Outside
        };
    }
    if !open {
        return FightPop::Outside;
    }
    let verdict = host.judge_pop(name, source, arguments);
    drop(host);
    match verdict {
        HostPop::Run => FightPop::HostWindow,
        HostPop::RunRequested => {
            log_line(&format!(
                "fight request: the guest's {name} runs here once and is streamed back"
            ));
            FightPop::HostWindow
        }
        HostPop::HoldSuperseded => {
            log_line(&format!(
                "fight request: the guest's {name} not run, your stream already carried that copy"
            ));
            FightPop::Hold
        }
    }
}

fn note_registry_kinds_for_the_guest(build: &VerifiedBuild, manager: u64) {
    let active = here(build)
        .filter(|here| here.role == FollowRole::Guest)
        .is_some_and(|here| {
            GUEST
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_active_in(here.local_phase)
        });
    let mut kinds = REGISTRY_KINDS_BEFORE_POP
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    kinds.clear();
    if !active {
        return;
    }
    // A pop frees a one-shot entry (kind -1), so the kind is read before the pop.
    let memory = InProcessMemory::new(build);
    kinds.extend((0..CALLBACK_REGISTRY_ENTRY_COUNT).map(|index| {
        memory
            .read_i32(callback_registry_kind_address(manager, index))
            .unwrap_or(-1)
    }));
}

fn fired_by_this_player(manager: u64, arguments: u64) -> bool {
    let Some(index) = fired_registry_index(manager, arguments) else {
        return false;
    };
    let kinds = REGISTRY_KINDS_BEFORE_POP
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    usize::try_from(index)
        .ok()
        .and_then(|index| kinds.get(index))
        .is_some_and(|kind| is_player_tested_callback_kind(*kind))
}

fn judge_guest_pop(
    manager: u64,
    name: &str,
    arguments: u64,
    phase: u16,
    queued_by_object: bool,
) -> FightPop {
    let Some(source) = pop_source(manager, arguments) else {
        return FightPop::Outside;
    };
    let more_local_ids = GUEST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .window()
        .map_or(&[][..], |window| window.player_local_messages);
    let registered_local = is_player_local_message_handler(phase, name, more_local_ids);
    let fired_by_player = source == PopSource::Fired && fired_by_this_player(manager, arguments);
    let verdict = if is_pop_by_this_player(fired_by_player, queued_by_object) {
        let args = BUILD
            .get()
            .and_then(|build| read_arguments(&InProcessMemory::new(build), arguments))
            .unwrap_or_default();
        GUEST
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_by_player(name, args, Some(phase), registered_local, net_clock_ms())
    } else {
        GUEST.lock().unwrap_or_else(PoisonError::into_inner).pop(
            name,
            source,
            arguments,
            Some(phase),
            registered_local,
        )
    };
    match verdict {
        GuestPop::RunReplayed { seq } => {
            log_line(&format!(
                "fight stream: the host's {name} (seq {seq}) runs here"
            ));
            FightPop::Replayed
        }
        GuestPop::RunEarlierEntry => {
            log_line(&format!(
                "fight stream: the host's {name}, queued before its fight moved on, runs here"
            ));
            FightPop::Replayed
        }
        GuestPop::RunPlayerLocal => {
            log_line(&format!(
                "fight stream: {name} is player-local, your game runs its own"
            ));
            FightPop::Outside
        }
        GuestPop::RunRefusedHere => {
            log_line(&format!(
                "fight request: your player's {name}, refused by the host, runs in your game"
            ));
            FightPop::Outside
        }
        GuestPop::RunStartEntry => {
            log_line(&format!(
                "fight stream: the host's start {name} (entry 0) runs here, before the fight's other entries"
            ));
            crate::cutscene_skip::forget_gate_copies(name);
            FightPop::Replayed
        }
        GuestPop::StartAlreadyRan => {
            log_line(&format!(
                "fight stream: another copy of the host's start {name} not run, it already ran here once in this fight"
            ));
            FightPop::Hold
        }
        GuestPop::RunPartnerCall | GuestPop::Outside => FightPop::Outside,
        GuestPop::Hold => {
            log_line(&format!(
                "fight stream: your game's {name} not run, the host's stream runs the fight"
            ));
            FightPop::Hold
        }
        GuestPop::HeldForHostCopy => {
            log_line(&format!(
                "fight request: your player set off {name}, held until the host's copy pairs with it, else it goes to the host to run once"
            ));
            FightPop::Hold
        }
    }
}
