use std::panic;
use std::sync::OnceLock;

use automata_game::story_follow::{FollowReadings, FollowSend, FollowStep};
use automata_game::story_gate::StoryRule;
use automata_proto::Message;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;
use nc_hook::net::net_clock_ms;

use crate::game_calls::GameCalls;
use crate::game_hooks::{run_as_ours, story_gate_is_armed, story_gate_is_hooked};
use crate::net::{handle, local_story_place};
use crate::start_driver::link_reading;
use crate::story_follow_driver::shared_machine::{start_machine, with_machine};

static CONTEXT: OnceLock<FollowContext> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
struct FollowContext {
    calls: GameCalls,
}

pub fn install(build: VerifiedBuild, rule: StoryRule) {
    if rule != StoryRule::Follow {
        return;
    }
    if !story_gate_is_hooked() {
        log_line("story: follow is set but the gate is not hooked, this game does not follow");
        return;
    }
    let Some(variant) = automata_game::build::variant_of_image_size(build.size_of_image()) else {
        return;
    };
    start_machine();
    let _ = CONTEXT.set(FollowContext {
        calls: GameCalls::new(build, variant),
    });
    log_line(
        "story: follow is on, the host leads the story and the guest's story steps wait for the host's",
    );
}

pub fn tick() {
    let Some(context) = CONTEXT.get() else {
        return;
    };
    let _ = panic::catch_unwind(|| context.step_the_machine());
}

impl FollowContext {
    fn step_the_machine(&self) {
        let armed = story_gate_is_armed();
        let link = link_reading();
        let (moves, asks, answers) = hear_the_partner(armed);
        let readings = FollowReadings {
            now_ms: net_clock_ms(),
            role: link.role,
            partner_present: armed && link.partner_present,
            can_take_a_request: self.calls.request_readiness().can_take_a_request(),
            current_place: local_story_place(),
            host_place: link.partner_place,
            host_report_heard_at_ms: link.partner.heard_at_ms,
            host_in_play: link.partner.in_play,
            host_waiting_at_load_end: link.partner.waiting,
            moves,
            asks,
            answers,
        };
        let Some(step) = with_machine(|machine| machine.step(&readings)) else {
            return;
        };
        self.act_on(step);
    }

    fn act_on(&self, step: FollowStep) {
        for line in &step.logs {
            log_line(line);
        }
        if let Some(net) = handle() {
            for send in step.sends {
                net.send_relayed(match send {
                    FollowSend::Move(story_move) => Message::StoryMove(story_move),
                    FollowSend::Ask(ask) => Message::StoryAsk(ask),
                    FollowSend::Answer(answer) => Message::StoryAskAnswer(answer),
                });
            }
        }
        if let Some(request) = step.make {
            run_as_ours(|| self.calls.make_story_request(&request));
        }
    }
}

fn hear_the_partner(
    armed: bool,
) -> (
    Vec<automata_game::story_follow::StoryMove>,
    Vec<automata_game::story_follow::StoryAsk>,
    Vec<automata_game::story_follow::StoryAskAnswer>,
) {
    let Some(net) = handle() else {
        return Default::default();
    };
    let heard = net.take_relayed_from(|message| {
        matches!(
            message,
            Message::StoryMove(_) | Message::StoryAsk(_) | Message::StoryAskAnswer(_)
        )
    });
    let mut moves = Vec::new();
    let mut asks = Vec::new();
    let mut answers = Vec::new();
    for (_, message) in heard {
        match message {
            Message::StoryMove(story_move) => moves.push(story_move),
            Message::StoryAsk(ask) => asks.push(ask),
            Message::StoryAskAnswer(answer) => answers.push(answer),
            _ => {}
        }
    }
    if !armed {
        // Heard before the start is over: the start brings this game to the host's place itself.
        return Default::default();
    }
    (moves, asks, answers)
}
