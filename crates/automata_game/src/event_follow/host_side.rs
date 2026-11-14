use crate::event_follow::{
    EventAsk, EventAskWhat, EventCall, EventId, EventReadings, EventSend, EventStep, PlayedHere,
    PlayingHere, RunsForThePartner, ask_already_played_line, ask_refused_line, ask_retold_line,
    ask_skip_ignored_line, ask_skip_made_line, ask_start_made_line, mirrored_event_playing,
};
use crate::travel::JumpRole;

/// The host's side: the guest's asks.
#[derive(Debug, Clone, Default)]
pub struct HostSide {
    last_refusal: Option<(EventId, &'static str)>,
}

impl HostSide {
    pub fn answer(
        &mut self,
        readings: &EventReadings,
        playing: &[PlayingHere],
        played: &PlayedHere,
        runs: &mut RunsForThePartner,
        starts_by_data_id: bool,
        step: &mut EventStep,
    ) {
        for ask in &readings.asks {
            match ask.what {
                EventAskWhat::Start => {
                    self.answer_start(
                        ask,
                        readings,
                        playing,
                        played,
                        runs,
                        starts_by_data_id,
                        step,
                    );
                }
                EventAskWhat::Skip if mirrored_event_playing(playing, ask.id) => {
                    step.calls.push(EventCall::Skip);
                    step.logs.push(ask_skip_made_line(ask.id));
                }
                EventAskWhat::Skip => step.logs.push(ask_skip_ignored_line(ask.id)),
                EventAskWhat::AlreadyPlayed => {}
            }
        }
    }

    fn answer_start(
        &mut self,
        ask: &EventAsk,
        readings: &EventReadings,
        playing: &[PlayingHere],
        played: &PlayedHere,
        runs: &mut RunsForThePartner,
        starts_by_data_id: bool,
        step: &mut EventStep,
    ) {
        let id = ask.id;
        if mirrored_event_playing(playing, id) {
            // The start was told already; the guest did not hear it.
            step.sends.push(EventSend::Started(id));
            step.logs.push(ask_retold_line(id));
            return;
        }
        if played.contains(id) {
            step.sends.push(EventSend::Ask(EventAsk {
                id,
                what: EventAskWhat::AlreadyPlayed,
            }));
            step.logs.push(ask_already_played_line(id));
            return;
        }
        if runs.has(id) {
            return;
        }
        let refusal = if !starts_by_data_id {
            Some("starts for the partner are turned off here")
        } else if readings.rule_role() != JumpRole::Host {
            Some("the host or the guest is not in play, or this is not a story phase")
        } else if !readings.in_the_partners_place() {
            Some("the host is at another place")
        } else if readings.plays_an_event() || !playing.is_empty() {
            Some("the host plays another event")
        } else {
            None
        };
        match refusal {
            Some(reason) => {
                if self.last_refusal != Some((id, reason)) {
                    self.last_refusal = Some((id, reason));
                    step.logs.push(ask_refused_line(id, reason));
                }
            }
            None => {
                self.last_refusal = None;
                runs.begin(id, readings.now_ms, step);
                step.logs.push(ask_start_made_line(id));
            }
        }
    }
}
