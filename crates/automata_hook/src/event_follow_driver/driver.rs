use std::panic;
use std::sync::OnceLock;

use automata_game::address::CURRENT_PHASE_ID_VALUE;
use automata_game::event_follow::{
    EventAsk, EventCall, EventId, EventReadings, EventSend, EventStep, told_line,
};
use automata_proto::Message;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;
use nc_hook::net::net_clock_ms;

use crate::event_follow_driver::shared_machine::{start_machine, with_machine};
use crate::game_calls::GameCalls;
use crate::game_hooks::{install_event_mirror, run_event_call_as_ours, story_gate_is_armed};
use crate::net::{
    handle, local_in_play, local_story_place, partner_playing_event_of, set_local_playing_event,
};
use crate::start_driver::link_reading;
use crate::start_setting::GameSettings;

const PHASE_ID_NOT_READ: i32 = -99;

static CONTEXT: OnceLock<EventContext> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
struct EventContext {
    calls: GameCalls,
}

pub fn install(build: VerifiedBuild, settings: &GameSettings) {
    if !settings.events_together {
        log_line("event: events_together is off, events are not mirrored");
        return;
    }
    let Some(variant) = automata_game::build::variant_of_image_size(build.size_of_image()) else {
        return;
    };
    let calls = GameCalls::new(build, variant);
    if !install_event_mirror(calls) {
        log_line("event: the event mirror is not fully hooked, events are not mirrored");
        return;
    }
    start_machine(settings.events_started_for_the_partner);
    let _ = CONTEXT.set(EventContext { calls });
    log_line(if settings.events_started_for_the_partner {
        "event: events together is on, the host's story events reach the guest and the guest's wait for the host's; events are started for the partner by their DataId"
    } else {
        "event: events together is on, the host's story events reach the guest and the guest's wait for the host's; starts for the partner by DataId are off"
    });
}

pub fn tick() {
    let Some(context) = CONTEXT.get() else {
        return;
    };
    let _ = panic::catch_unwind(|| context.step_the_machine());
}

impl EventContext {
    fn step_the_machine(&self) {
        let armed = story_gate_is_armed();
        let link = link_reading();
        let (started_heard, ended_heard, asks) = hear_the_partner(armed);
        let partner_playing =
            handle().and_then(|net| net.update(|shared| partner_playing_event_of(shared)));
        let readings = EventReadings {
            now_ms: net_clock_ms(),
            role: link.role,
            partner_present: armed && link.partner_present,
            local_in_play: local_in_play(),
            partner_in_play: link.partner.in_play,
            current_phase_id: self
                .calls
                .read_i32(CURRENT_PHASE_ID_VALUE)
                .unwrap_or(PHASE_ID_NOT_READ),
            current_place: local_story_place(),
            partner_place: link.partner_place,
            partner_playing,
            slots: self.calls.event_slots(),
            event_flags_say_playing: self.calls.event_flags_say_playing(),
            started_heard,
            ended_heard,
            asks,
        };
        let Some(step) = with_machine(|machine| machine.step(&readings)) else {
            return;
        };
        set_local_playing_event(step.playing_story_event);
        self.act_on(step);
    }

    fn act_on(&self, step: EventStep) {
        for line in &step.logs {
            log_line(line);
        }
        if let Some(net) = handle() {
            for send in step.sends {
                let message = match send {
                    EventSend::Started(id) => {
                        log_line(&told_line(id, None));
                        Message::EventStarted { id }
                    }
                    EventSend::Ended { id, skipped } => {
                        log_line(&told_line(id, Some(skipped)));
                        Message::EventEnded { id, skipped }
                    }
                    EventSend::Ask(ask) => Message::EventAsk(ask),
                };
                net.send_relayed(message);
            }
        }
        for call in step.calls {
            run_event_call_as_ours(|| self.make(call));
        }
    }

    fn make(&self, call: EventCall) {
        match call {
            EventCall::ReleaseHeldPlay(id) => self.release_held_play(id),
            EventCall::Load(id) => self.calls.load_event(id),
            EventCall::Play(id) => self.calls.play_event(id),
            EventCall::Release(id) => self.calls.release_event(id),
            EventCall::Skip => {
                if !self.calls.skip_playing_events() {
                    log_line("event: a skip was due but no event plays here, nothing skipped");
                }
            }
        }
    }

    fn release_held_play(&self, id: EventId) {
        // A slot the script let go of is not brought back: the call would load the event anew.
        if self.calls.event_slots().iter().any(|slot| slot.id == id) {
            self.calls.play_event(id);
        } else {
            log_line(&format!(
                "event: {} was released but its slot is gone, nothing is played",
                id.describe()
            ));
        }
    }
}

fn hear_the_partner(armed: bool) -> (Vec<EventId>, Vec<(EventId, bool)>, Vec<EventAsk>) {
    let Some(net) = handle() else {
        return Default::default();
    };
    let heard = net.take_relayed_from(|message| {
        matches!(
            message,
            Message::EventStarted { .. } | Message::EventEnded { .. } | Message::EventAsk(_)
        )
    });
    if !armed {
        return Default::default();
    }
    let mut started = Vec::new();
    let mut ended = Vec::new();
    let mut asks = Vec::new();
    for (_, message) in heard {
        match message {
            Message::EventStarted { id } => started.push(id),
            Message::EventEnded { id, skipped } => ended.push((id, skipped)),
            Message::EventAsk(ask) => asks.push(ask),
            _ => {}
        }
    }
    (started, ended, asks)
}
