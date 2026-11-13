use crate::event_follow::{
    EVENT_STARTED_FOR_THE_PARTNER_LOAD_GIVE_UP_MS, EVENT_STARTED_FOR_THE_PARTNER_PLAY_GIVE_UP_MS,
    EVENT_STARTED_FOR_THE_PARTNER_WATCHED_MOST_MS, EventCall, EventId, EventReadings, EventStep,
    SeenEvents, run_gave_up_line, run_released_line,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunStage {
    Loading,
    PlayAsked,
    Playing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Run {
    id: EventId,
    stage: RunStage,
    since_ms: u64,
}

/// Events this copy starts by their DataId: load, wait until loaded, play, release after the end.
#[derive(Debug, Clone, Default)]
pub struct RunsForThePartner {
    runs: Vec<Run>,
}

impl RunsForThePartner {
    pub fn has(&self, id: EventId) -> bool {
        self.runs.iter().any(|run| run.id == id)
    }

    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }

    pub fn begin(&mut self, id: EventId, now_ms: u64, step: &mut EventStep) {
        if self.has(id) {
            return;
        }
        self.runs.push(Run {
            id,
            stage: RunStage::Loading,
            since_ms: now_ms,
        });
        step.calls.push(EventCall::Load(id));
    }

    /// True when this start is the run's own (or the script's play of an event a run loads).
    pub fn take_start(&mut self, id: EventId, now_ms: u64) -> bool {
        let Some(run) = self
            .runs
            .iter_mut()
            .find(|run| run.id == id && run.stage != RunStage::Playing)
        else {
            return false;
        };
        run.stage = RunStage::Playing;
        run.since_ms = now_ms;
        true
    }

    pub fn end(&mut self, id: EventId, step: &mut EventStep) {
        let before = self.runs.len();
        self.runs.retain(|run| run.id != id);
        if self.runs.len() != before {
            step.calls.push(EventCall::Release(id));
            step.logs.push(run_released_line(id));
        }
    }

    pub fn advance(
        &mut self,
        readings: &EventReadings,
        seen: &mut SeenEvents,
        step: &mut EventStep,
    ) {
        let now_ms = readings.now_ms;
        let mut kept = Vec::with_capacity(self.runs.len());
        for mut run in self.runs.drain(..) {
            let waited_ms = now_ms.saturating_sub(run.since_ms);
            let slot = readings.slot_of(run.id);
            let gave_up = match run.stage {
                RunStage::Loading if slot.is_some_and(|slot| slot.is_loaded()) => {
                    step.calls.push(EventCall::Play(run.id));
                    seen.remember(run.id, readings.current_place, now_ms);
                    run.stage = RunStage::PlayAsked;
                    run.since_ms = now_ms;
                    None
                }
                RunStage::Loading if slot.is_some_and(|slot| slot.load_failed()) => {
                    Some("its load failed")
                }
                RunStage::Loading if waited_ms >= EVENT_STARTED_FOR_THE_PARTNER_LOAD_GIVE_UP_MS => {
                    Some("it did not load in time")
                }
                RunStage::PlayAsked
                    if waited_ms >= EVENT_STARTED_FOR_THE_PARTNER_PLAY_GIVE_UP_MS =>
                {
                    Some("it did not start in time")
                }
                RunStage::Playing if waited_ms >= EVENT_STARTED_FOR_THE_PARTNER_WATCHED_MOST_MS => {
                    Some("it played longer than it is watched")
                }
                _ => None,
            };
            match gave_up {
                Some(reason) => {
                    step.calls.push(EventCall::Release(run.id));
                    step.logs.push(run_gave_up_line(run.id, waited_ms, reason));
                }
                None => kept.push(run),
            }
        }
        self.runs = kept;
    }
}
