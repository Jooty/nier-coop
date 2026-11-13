use std::panic;
use std::sync::{Mutex, OnceLock};

use automata_game::partner::not_free_reason;
use automata_game::save_together::{GuestGates, SaveMachine, SaveReadings, guest_asked_slot_line};
use automata_proto::Message;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;
use nc_hook::net::net_clock_ms;

use crate::game_calls::GameCalls;
use crate::game_hooks::{
    install_save_request_watch, last_slot_saved, run_as_ours, take_saves_asked_by_the_game,
};
use crate::net::{handle, local_in_play};
use crate::start_driver::link_reading;
use crate::start_setting::GameSettings;

static CONTEXT: OnceLock<SaveContext> = OnceLock::new();
static MACHINE: Mutex<Option<SaveMachine>> = Mutex::new(None);

#[derive(Debug, Clone, Copy)]
struct SaveContext {
    calls: GameCalls,
    configured_slot: u32,
}

pub fn install(build: VerifiedBuild, settings: &GameSettings) {
    if !settings.save_together {
        log_line(
            "save together: save_together is off, the guest's saves are not tied to the host's",
        );
        return;
    }
    let Some(variant) = automata_game::build::variant_of_image_size(build.size_of_image()) else {
        return;
    };
    let calls = GameCalls::new(build, variant);
    if !install_save_request_watch(build, &calls) {
        log_line("save together: the save request is not hooked, saves are not tied together");
        return;
    }
    if let Ok(mut guard) = MACHINE.lock() {
        *guard = Some(SaveMachine::new());
    }
    let _ = CONTEXT.set(SaveContext {
        calls,
        configured_slot: settings.slot,
    });
    log_line(
        "save together: on, the host's slot saves make the guest save its own current slot when its player is free",
    );
}

pub fn tick() {
    let Some(context) = CONTEXT.get() else {
        return;
    };
    let _ = panic::catch_unwind(|| context.step_the_machine());
}

impl SaveContext {
    fn step_the_machine(&self) {
        let Ok(mut guard) = MACHINE.lock() else {
            return;
        };
        let Some(machine) = guard.as_mut() else {
            return;
        };
        let link = link_reading();
        let readings = SaveReadings {
            now_ms: net_clock_ms(),
            role: link.role,
            campaign_partner_present: link.partner_present,
            local_in_play: local_in_play(),
            partner_in_play: link.partner.in_play,
            saves_asked_by_the_game: take_saves_asked_by_the_game(),
            host_saves_heard: hear_the_host(),
            gates: machine.wants_gates().then(|| self.read_gates()),
        };
        let step = machine.step(&readings);
        for line in &step.logs {
            log_line(line);
        }
        if let Some(net) = handle() {
            for number in step.tell {
                net.send_relayed(Message::HostSaved { number });
            }
        }
        if step.request_save {
            let slot = last_slot_saved().unwrap_or(self.configured_slot);
            log_line(&guest_asked_slot_line(slot));
            run_as_ours(|| self.calls.request_save_slot(slot));
            machine.save_requested(self.calls.save_is_busy());
        }
    }

    fn read_gates(&self) -> GuestGates {
        let busy = self.calls.save_is_busy();
        GuestGates {
            player_held_by: not_free_reason(&self.calls.read_free())
                .map(|reason| reason.to_string()),
            save_busy: Some(busy),
            save_failed: (!busy).then(|| self.calls.save_failed()).flatten(),
        }
    }
}

fn hear_the_host() -> Vec<u32> {
    let Some(net) = handle() else {
        return Vec::new();
    };
    net.take_relayed_from(|message| matches!(message, Message::HostSaved { .. }))
        .into_iter()
        .filter_map(|(_, message)| match message {
            Message::HostSaved { number } => Some(number),
            _ => None,
        })
        .collect()
}
