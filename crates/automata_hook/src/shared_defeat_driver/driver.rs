use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use automata_game::shared_defeat::{
    DefeatNote, GAME_OVER_MENU_RETRY_CHOSEN_STATE, GameOverMenuWrite, RETRY_PHASE_CHANGE_PARAMS,
    SharedDefeatMachine, SharedDefeatReadings, menu_write_done_line, shared_defeat_off_line,
    shared_defeat_on_line,
};
use automata_game::story_follow::StoryRequest;
use automata_proto::Message;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;
use nc_hook::net::net_clock_ms;

use crate::game_calls::GameCalls;
use crate::game_hooks::run_as_ours;
use crate::net::{handle, local_unattended};
use crate::shared_defeat_driver::body_guard::BodyGuard;
use crate::start_driver::link_reading;
use crate::start_setting::GameSettings;

static CONTEXT: OnceLock<DefeatContext> = OnceLock::new();
static MACHINE: Mutex<Option<SharedDefeatMachine>> = Mutex::new(None);
static BODY_GUARD: Mutex<BodyGuard> = Mutex::new(BodyGuard::new());
static NOTES_ARE_ONLY_DROPPED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy)]
struct DefeatContext {
    calls: GameCalls,
}

pub fn install(build: VerifiedBuild, settings: &GameSettings) {
    if !settings.shared_defeat {
        NOTES_ARE_ONLY_DROPPED.store(true, Ordering::Relaxed);
        log_line(&shared_defeat_off_line());
        return;
    }
    let Some(variant) = automata_game::build::variant_of_image_size(build.size_of_image()) else {
        NOTES_ARE_ONLY_DROPPED.store(true, Ordering::Relaxed);
        return;
    };
    if let Ok(mut guard) = MACHINE.lock() {
        *guard = Some(SharedDefeatMachine::new());
    }
    let _ = CONTEXT.set(DefeatContext {
        calls: GameCalls::new(build, variant),
    });
    log_line(&shared_defeat_on_line());
}

pub fn tick() {
    let Some(context) = CONTEXT.get() else {
        if NOTES_ARE_ONLY_DROPPED.load(Ordering::Relaxed) {
            // A partner with the stage on still sends notes; they must not crowd the inbox.
            let _ = hear_notes();
        }
        return;
    };
    let _ = panic::catch_unwind(|| context.step_the_machine());
}

impl DefeatContext {
    fn step_the_machine(&self) {
        let Ok(mut guard) = MACHINE.lock() else {
            return;
        };
        let Some(machine) = guard.as_mut() else {
            return;
        };
        let calls = &self.calls;
        let link = link_reading();
        let flow = calls.read_defeat_flow();
        let readings = SharedDefeatReadings {
            now_ms: net_clock_ms(),
            role: link.role,
            campaign_partner_present: link.partner_present,
            flow,
            menu: flow
                .is_some_and(|flow| flow.is_active())
                .then(|| calls.read_game_over_menu())
                .flatten(),
            in_place_retry: calls.in_place_retry_is_set(),
            restore_point: calls.read_restore_point(),
            restore_place: calls.read_restore_place(),
            heard: hear_notes(),
            local_unattended: local_unattended(),
        };
        let step = machine.step(&readings);
        for line in &step.logs {
            log_line(line);
        }
        if let Some(net) = handle() {
            for note in step.tell {
                net.send_relayed(Message::DefeatNote(note));
            }
        }
        if let Some(write) = step.menu_write {
            log_line(&menu_write_done_line(write, self.make(write)));
        }
        if readings.campaign_pair()
            && let Ok(mut body_guard) = BODY_GUARD.lock()
        {
            body_guard.keep_the_partner_body_out_of_this_copys_flow(calls);
        }
    }

    fn make(&self, write: GameOverMenuWrite) -> bool {
        let calls = &self.calls;
        match write {
            GameOverMenuWrite::Title => calls.write_game_over_menu_title(),
            GameOverMenuWrite::RetryInPlace => {
                let readiness = calls.request_readiness();
                let Some(name) = readiness.current_name else {
                    return false;
                };
                if readiness.current_phase_id <= 0 {
                    return false;
                }
                let request = StoryRequest::Full {
                    id: readiness.current_phase_id,
                    name,
                    params: RETRY_PHASE_CHANGE_PARAMS,
                };
                run_as_ours(|| calls.make_story_request(&request));
                calls.write_game_over_menu_state(GAME_OVER_MENU_RETRY_CHOSEN_STATE)
            }
        }
    }
}

fn hear_notes() -> Vec<DefeatNote> {
    let Some(net) = handle() else {
        return Vec::new();
    };
    net.take_relayed_from(|message| matches!(message, Message::DefeatNote(_)))
        .into_iter()
        .filter_map(|(_, message)| match message {
            Message::DefeatNote(note) => Some(note),
            _ => None,
        })
        .collect()
}
