use crate::event_follow::{
    EventAsk, EventAskWhat, EventCall, EventId, EventReadings, EventSend, EventStep,
    HOST_EVENT_START_REMEMBERED_MS, MOST_HOST_EVENTS_REMEMBERED, PlayingHere, RunsForThePartner,
    ask_refused_line, ask_retold_line, ask_skip_ignored_line, ask_skip_made_line,
    ask_start_made_line, story_event_playing,
};
use crate::travel::JumpRole;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ToldStart {
    id: EventId,
    started_at_ms: u64,
    ended_skipped: Option<bool>,
}

/// The host's side: its own told starts and ends, and the guest's asks.
#[derive(Debug, Clone, Default)]
pub struct HostSide {
    told: Vec<ToldStart>,
    last_refusal: Option<(EventId, &'static str)>,
}

impl HostSide {
    pub fn started(&mut self, id: EventId, now_ms: u64) {
        self.told.retain(|start| start.id != id);
        if self.told.len() >= MOST_HOST_EVENTS_REMEMBERED {
            self.told.remove(0);
        }
        self.told.push(ToldStart {
            id,
            started_at_ms: now_ms,
            ended_skipped: None,
        });
    }

    pub fn ended(&mut self, id: EventId, skipped: bool) {
        if let Some(start) = self.told.iter_mut().find(|start| start.id == id) {
            start.ended_skipped = Some(skipped);
        }
    }

    pub fn answer(
        &mut self,
        readings: &EventReadings,
        playing: &[PlayingHere],
        runs: &mut RunsForThePartner,
        starts_by_data_id: bool,
        step: &mut EventStep,
    ) {
        let now_ms = readings.now_ms;
        self.told.retain(|start| {
            start.ended_skipped.is_none()
                || now_ms.saturating_sub(start.started_at_ms) <= HOST_EVENT_START_REMEMBERED_MS
        });
        for ask in &readings.asks {
            match ask.what {
                EventAskWhat::Start => {
                    self.answer_start(ask, readings, playing, runs, starts_by_data_id, step);
                }
                EventAskWhat::Skip if story_event_playing(playing, ask.id) => {
                    step.calls.push(EventCall::Skip);
                    step.logs.push(ask_skip_made_line(ask.id));
                }
                EventAskWhat::Skip => step.logs.push(ask_skip_ignored_line(ask.id)),
            }
        }
    }

    fn answer_start(
        &mut self,
        ask: &EventAsk,
        readings: &EventReadings,
        playing: &[PlayingHere],
        runs: &mut RunsForThePartner,
        starts_by_data_id: bool,
        step: &mut EventStep,
    ) {
        let id = ask.id;
        if story_event_playing(playing, id) {
            // The start was told already; the guest did not hear it.
            step.sends.push(EventSend::Started(id));
            step.logs.push(ask_retold_line(id, None));
            return;
        }
        if let Some(start) = self.told.iter().find(|start| start.id == id)
            && let Some(skipped) = start.ended_skipped
        {
            step.sends.push(EventSend::Started(id));
            step.sends.push(EventSend::Ended { id, skipped });
            step.logs.push(ask_retold_line(id, Some(skipped)));
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
