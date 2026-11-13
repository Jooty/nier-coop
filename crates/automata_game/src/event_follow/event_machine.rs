use crate::event_follow::guest_side::GuestSide;
use crate::event_follow::host_side::HostSide;
use crate::event_follow::{
    EventCall, EventId, EventReadings, EventSend, EventStep, MOST_SCRIPT_PLAY_REQUESTS_REMEMBERED,
    PlayVerdict, PlayingHere, RunsForThePartner, SCRIPT_PLAY_REQUEST_COUNTS_FOR_ITS_START_MS,
    SeenEvents, SkipVerdict, StartOrigin, ended_here_line, replay_skipped_line, started_here_line,
};
use crate::load_gate::StoryPlace;
use crate::travel::JumpRole;

/// Starts, ends and skips of events, mirrored between the two copies of a campaign.
#[derive(Debug, Clone, Default)]
pub struct EventMachine {
    starts_by_data_id: bool,
    linked_role: JumpRole,
    rule_role: JumpRole,
    place: StoryPlace,
    playing: Vec<PlayingHere>,
    script_asked: Vec<(EventId, u64)>,
    seen: SeenEvents,
    runs: RunsForThePartner,
    host: HostSide,
    guest: GuestSide,
    before_the_next_step: EventStep,
}

impl EventMachine {
    pub fn new(starts_by_data_id: bool) -> Self {
        Self {
            starts_by_data_id,
            ..Self::default()
        }
    }

    /// The story script asked for an event to play (the call is not ours).
    pub fn on_play_request(&mut self, id: EventId, now_ms: u64) -> PlayVerdict {
        self.script_asked.retain(|(asked, _)| *asked != id);
        if self.script_asked.len() >= MOST_SCRIPT_PLAY_REQUESTS_REMEMBERED {
            self.script_asked.remove(0);
        }
        self.script_asked.push((id, now_ms));
        if self.rule_role != JumpRole::Guest
            || self.seen.contains(id, &self.place, now_ms)
            || self.playing.iter().any(|here| here.id == id)
        {
            return PlayVerdict::Pass;
        }
        self.guest
            .play_request(id, now_ms, &mut self.before_the_next_step.logs)
    }

    pub fn on_start(&mut self, id: EventId, now_ms: u64) {
        let step = &mut self.before_the_next_step;
        let origin = if self.runs.take_start(id, now_ms) {
            StartOrigin::ForThePartner
        } else if self.seen.contains(id, &self.place, now_ms) {
            StartOrigin::Replay
        } else if self.script_asked.iter().any(|(asked, at_ms)| {
            *asked == id
                && now_ms.saturating_sub(*at_ms) <= SCRIPT_PLAY_REQUEST_COUNTS_FOR_ITS_START_MS
        }) {
            StartOrigin::Script
        } else {
            StartOrigin::Other
        };
        self.script_asked.retain(|(asked, _)| *asked != id);
        let told = origin.is_a_story_event() && self.linked_role == JumpRole::Host;
        self.playing.retain(|here| here.id != id);
        self.playing.push(PlayingHere {
            id,
            since_ms: now_ms,
            origin,
            told,
        });
        step.logs.push(started_here_line(id, origin));
        if origin == StartOrigin::Replay {
            step.calls.push(EventCall::Skip);
            step.logs.push(replay_skipped_line(id));
        }
        if told {
            self.host.started(id, now_ms);
            step.sends.push(EventSend::Started(id));
        }
        if origin.is_a_story_event() && self.linked_role == JumpRole::Guest {
            self.guest.own_start(id, now_ms, step);
        }
    }

    pub fn on_end(&mut self, id: EventId, skipped: bool, now_ms: u64) {
        let step = &mut self.before_the_next_step;
        let ended = self
            .playing
            .iter()
            .position(|here| here.id == id)
            .map(|index| self.playing.remove(index));
        step.logs.push(ended_here_line(
            id,
            skipped,
            ended.map(|here| now_ms.saturating_sub(here.since_ms)),
        ));
        if ended.is_some_and(|here| here.told) && self.linked_role == JumpRole::Host {
            self.host.ended(id, skipped);
            step.sends.push(EventSend::Ended { id, skipped });
        }
        self.runs.end(id, step);
        self.guest.own_end(id);
    }

    /// The player chose skip in the event pause (the call is not ours).
    pub fn on_player_skip(&mut self, now_ms: u64) -> SkipVerdict {
        if self.rule_role != JumpRole::Guest {
            return SkipVerdict::Pass;
        }
        self.guest
            .player_skip(&self.playing, now_ms, &mut self.before_the_next_step)
    }

    pub fn step(&mut self, readings: &EventReadings) -> EventStep {
        let mut step = std::mem::take(&mut self.before_the_next_step);
        let now_ms = readings.now_ms;
        let linked_role = readings.linked_role();
        let rule_role = readings.rule_role();
        if self.rule_role == JumpRole::Guest && rule_role != JumpRole::Guest {
            self.guest.release_all(now_ms, &mut step);
        }
        if self.linked_role == JumpRole::Host && linked_role != JumpRole::Host {
            self.host = HostSide::default();
        }
        if self.linked_role == JumpRole::Guest && linked_role != JumpRole::Guest {
            self.guest = GuestSide::default();
        }
        self.linked_role = linked_role;
        self.rule_role = rule_role;
        self.place = readings.current_place;
        self.script_asked.retain(|(_, at_ms)| {
            now_ms.saturating_sub(*at_ms) <= SCRIPT_PLAY_REQUEST_COUNTS_FOR_ITS_START_MS
        });
        self.seen.forget_old(now_ms);
        match linked_role {
            JumpRole::Host => self.host.answer(
                readings,
                &self.playing,
                &mut self.runs,
                self.starts_by_data_id,
                &mut step,
            ),
            JumpRole::Guest => self.guest.hear(readings, &self.playing, &mut step),
            JumpRole::Solo => {}
        }
        if rule_role == JumpRole::Guest {
            self.guest.step_in_rule(
                readings,
                &self.playing,
                &mut self.runs,
                &self.seen,
                self.starts_by_data_id,
                &mut step,
            );
        }
        self.runs.advance(readings, &mut self.seen, &mut step);
        if linked_role == JumpRole::Solo {
            step.sends.clear();
        }
        if rule_role != JumpRole::Guest {
            step.sends.retain(|send| !matches!(send, EventSend::Ask(_)));
        }
        step.playing_story_event = self
            .playing
            .iter()
            .rev()
            .find(|here| here.origin.is_a_story_event())
            .map(|here| here.id);
        step
    }
}
