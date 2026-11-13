use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use automata_game::story_steps::{
    ActionLook, StepClassLookup, StepSend, StepsReadings, StepsStep, StoryStepAnswer, StoryStepAsk,
    StoryStepDone, busy_passes_line, set_done_line, story_steps_hooks_missing_line,
    story_steps_off_line, story_steps_on_line,
};
use automata_proto::Message;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;
use nc_hook::net::net_clock_ms;

use crate::game_calls::GameCalls;
use crate::game_hooks::{
    install_story_step_hooks, run_as_ours, set_story_steps_watching, story_gate_is_armed,
};
use crate::net::{handle, local_story_place};
use crate::start_driver::link_reading;
use crate::start_setting::GameSettings;
use crate::story_steps_driver::shared_machine::{
    start_machine, stop_machine, take_unseen_while_busy, with_machine,
};

static CONTEXT: OnceLock<StepsContext> = OnceLock::new();
static HEARD_IS_ONLY_DROPPED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy)]
struct StepsContext {
    calls: GameCalls,
}

#[derive(Debug, Default)]
struct Heard {
    told: Vec<StoryStepDone>,
    asks: Vec<StoryStepAsk>,
    answers: Vec<StoryStepAnswer>,
}

pub fn install(build: VerifiedBuild, settings: &GameSettings) {
    let mode = settings.story_steps;
    if !mode.installs_hooks() {
        HEARD_IS_ONLY_DROPPED.store(true, Ordering::Relaxed);
        log_line(&story_steps_off_line());
        return;
    }
    let Some(variant) = automata_game::build::variant_of_image_size(build.size_of_image()) else {
        HEARD_IS_ONLY_DROPPED.store(true, Ordering::Relaxed);
        return;
    };
    let calls = GameCalls::new(build, variant);
    let lookup = StepClassLookup::for_variant(variant);
    let classes = lookup.len();
    start_machine(mode);
    if !install_story_step_hooks(&calls, lookup, mode.holds_and_applies()) {
        stop_machine();
        HEARD_IS_ONLY_DROPPED.store(true, Ordering::Relaxed);
        log_line(&story_steps_hooks_missing_line());
        return;
    }
    let _ = CONTEXT.set(StepsContext { calls });
    log_line(&story_steps_on_line(mode, classes));
}

pub fn tick() {
    let Some(context) = CONTEXT.get() else {
        if HEARD_IS_ONLY_DROPPED.load(Ordering::Relaxed) {
            // A partner with the stage on still sends; its messages must not crowd the inbox.
            let _ = hear_the_partner();
        }
        return;
    };
    let _ = panic::catch_unwind(|| context.step_the_machine());
}

impl StepsContext {
    fn step_the_machine(&self) {
        let link = link_reading();
        let partner_present = story_gate_is_armed() && link.partner_present;
        let heard = hear_the_partner();
        let looks = if partner_present {
            self.look_at(&heard)
        } else {
            Vec::new()
        };
        let readings = StepsReadings {
            now_ms: net_clock_ms(),
            role: link.role,
            partner_present,
            place: local_story_place(),
            partner_place: link.partner_place,
            told: heard.told,
            asks: heard.asks,
            answers: heard.answers,
            looks,
        };
        let Some((step, acting)) =
            with_machine(|machine| (machine.step(&readings), machine.is_acting()))
        else {
            return;
        };
        set_story_steps_watching(acting);
        self.act_on(step);
        let unseen = take_unseen_while_busy();
        if unseen > 0 {
            log_line(&busy_passes_line(unseen));
        }
    }

    fn look_at(&self, heard: &Heard) -> Vec<ActionLook> {
        let mut ids = with_machine(|machine| machine.ids_to_look_at()).unwrap_or_default();
        ids.extend(heard.told.iter().map(|done| done.id));
        ids.extend(heard.asks.iter().map(|ask| ask.id));
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter()
            .map(|id| self.calls.look_at_story_action(id))
            .collect()
    }

    fn act_on(&self, step: StepsStep) {
        for line in &step.logs {
            log_line(line);
        }
        if let Some(net) = handle() {
            for send in step.sends {
                net.send_relayed(match send {
                    StepSend::Done(done) => Message::StoryStepDone(done),
                    StepSend::Ask(ask) => Message::StoryStepAsk(ask),
                    StepSend::Answer(answer) => Message::StoryStepAnswer(answer),
                });
            }
        }
        for set in &step.set_done {
            // The machine lock is free here: the detour sees this change as ours.
            let applied = run_as_ours(|| self.calls.set_story_action_done(set.id, set.value));
            log_line(&set_done_line(set, applied));
        }
    }
}

fn hear_the_partner() -> Heard {
    let Some(net) = handle() else {
        return Heard::default();
    };
    let mut heard = Heard::default();
    for (_, message) in net.take_relayed_from(|message| {
        matches!(
            message,
            Message::StoryStepDone(_) | Message::StoryStepAsk(_) | Message::StoryStepAnswer(_)
        )
    }) {
        match message {
            Message::StoryStepDone(done) => heard.told.push(done),
            Message::StoryStepAsk(ask) => heard.asks.push(ask),
            Message::StoryStepAnswer(answer) => heard.answers.push(answer),
            _ => {}
        }
    }
    heard
}
