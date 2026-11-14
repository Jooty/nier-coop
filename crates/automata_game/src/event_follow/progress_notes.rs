use crate::event_follow::{
    EVENT_PROGRESS_LINE_INTERVAL_MS, EventId, EventReadings, MOST_PARTNER_STARTS_NOTED,
    PlayingHere, event_progress_line,
};
use crate::travel::JumpRole;

/// When the other copy's events started, and the once-a-second progress line of the event here.
#[derive(Debug, Clone, Default)]
pub struct ProgressNotes {
    partner_started: Vec<(EventId, u64)>,
    last_line_ms: Option<u64>,
}

impl ProgressNotes {
    pub fn forget(&mut self, id: EventId) {
        self.partner_started.retain(|(noted, _)| *noted != id);
    }

    pub fn follow(
        &mut self,
        readings: &EventReadings,
        playing: &[PlayingHere],
        logs: &mut Vec<String>,
    ) {
        let now_ms = readings.now_ms;
        if readings.linked_role() == JumpRole::Solo {
            self.partner_started.clear();
        } else {
            for id in readings
                .started_heard
                .iter()
                .chain(&readings.partner_playing)
            {
                self.note_partner_start(*id, now_ms);
            }
        }
        let Some(progress) = readings.progress else {
            self.last_line_ms = None;
            return;
        };
        if self
            .last_line_ms
            .is_some_and(|at| now_ms.saturating_sub(at) < EVENT_PROGRESS_LINE_INTERVAL_MS)
        {
            return;
        }
        self.last_line_ms = Some(now_ms);
        let started_here_ms = playing
            .iter()
            .find(|here| here.id == progress.id)
            .map(|here| here.since_ms);
        let host_started_ms = self
            .partner_started
            .iter()
            .find(|(id, _)| *id == progress.id)
            .map(|(_, at_ms)| *at_ms);
        let earlier_by_ms = started_here_ms
            .zip(host_started_ms)
            .map(|(here, host)| here as i64 - host as i64);
        logs.push(event_progress_line(
            &progress,
            started_here_ms.map(|at_ms| now_ms.saturating_sub(at_ms)),
            earlier_by_ms,
        ));
    }

    fn note_partner_start(&mut self, id: EventId, now_ms: u64) {
        if self.partner_started.iter().any(|(noted, _)| *noted == id) {
            return;
        }
        if self.partner_started.len() >= MOST_PARTNER_STARTS_NOTED {
            self.partner_started.remove(0);
        }
        self.partner_started.push((id, now_ms));
    }
}
