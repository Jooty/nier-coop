use crate::event_follow::{
    EVENT_ASK_RESEND_MS, EVENT_FOLLOW_GRACE_MS, EVENT_FOLLOW_WAIT_LOG_INTERVAL_MS,
    EVENT_SEEN_REMEMBERED_MS, EventAsk, EventAskWhat, EventCall, EventId, EventReadings, EventSend,
    EventStep, GUEST_EVENT_MAY_OUTLAST_THE_HOSTS_BY_MS, GUEST_EVENT_START_GIVE_UP_MS,
    GUEST_SKIP_ASK_GIVE_UP_MS, HOST_EVENT_START_REMEMBERED_MS, MOST_HOST_EVENTS_REMEMBERED,
    MOST_PLAYS_HELD, PlayVerdict, PlayedHere, PlayingHere, RunsForThePartner, SeenEvents,
    SkipVerdict, already_played_heard_line, follow_start_line, follow_waits_line,
    held_gave_up_line, held_released_line, held_waiting_line, host_end_heard_line,
    host_start_heard_line, link_lost_line, mirrored_event_playing, outlast_skip_line,
    outlast_started_line, play_held_line, play_passed_line, skip_ask_gave_up_line,
    skip_swallowed_line, skipped_for_the_host_line,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HostEvent {
    id: EventId,
    started_at_ms: u64,
    ended: Option<(u64, bool)>,
    followed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeldPlay {
    id: EventId,
    since_ms: u64,
    asked_at_ms: Option<u64>,
}

/// The guest's side: what it heard of the host's events, its held plays, its skip ask.
#[derive(Debug, Clone, Default)]
pub struct GuestSide {
    host_events: Vec<HostEvent>,
    held: Vec<HeldPlay>,
    skip_ask: Option<(EventId, u64)>,
    outlasts: Vec<(EventId, u64)>,
    follow_wait_logged_at_ms: Option<u64>,
}

impl GuestSide {
    pub fn host_plays(&self, id: EventId) -> bool {
        self.host_events
            .iter()
            .any(|event| event.id == id && event.ended.is_none())
    }

    pub fn host_started_recently(&self, id: EventId, now_ms: u64) -> bool {
        self.host_events.iter().any(|event| {
            event.id == id
                && now_ms.saturating_sub(event.started_at_ms) <= HOST_EVENT_START_REMEMBERED_MS
        })
    }

    pub fn holds(&self, id: EventId) -> bool {
        self.held.iter().any(|held| held.id == id)
    }

    pub fn play_request(
        &mut self,
        id: EventId,
        now_ms: u64,
        logs: &mut Vec<String>,
    ) -> PlayVerdict {
        if self.host_plays(id) || self.host_started_recently(id, now_ms) {
            logs.push(play_passed_line(id));
            return PlayVerdict::Pass;
        }
        if self.holds(id) {
            return PlayVerdict::Hold;
        }
        if self.held.len() >= MOST_PLAYS_HELD {
            return PlayVerdict::Pass;
        }
        self.held.push(HeldPlay {
            id,
            since_ms: now_ms,
            asked_at_ms: None,
        });
        logs.push(play_held_line(id));
        PlayVerdict::Hold
    }

    pub fn own_start(&mut self, id: EventId, now_ms: u64, step: &mut EventStep) {
        self.held.retain(|held| held.id != id);
        let Some((_, skipped)) = self.host_end_of(id) else {
            return;
        };
        self.after_the_hosts_end(id, skipped, now_ms, step);
    }

    pub fn own_end(&mut self, id: EventId) {
        self.outlasts.retain(|(outlasting, _)| *outlasting != id);
        if self.skip_ask.is_some_and(|(asked, _)| asked == id) {
            self.skip_ask = None;
        }
    }

    pub fn player_skip(
        &mut self,
        playing: &[PlayingHere],
        now_ms: u64,
        step: &mut EventStep,
    ) -> SkipVerdict {
        let Some(shared) = playing
            .iter()
            .find(|here| here.origin.is_mirrored() && self.host_plays(here.id))
        else {
            return SkipVerdict::Pass;
        };
        if self.skip_ask.is_none_or(|(asked, _)| asked != shared.id) {
            self.skip_ask = Some((shared.id, now_ms));
            step.sends.push(EventSend::Ask(EventAsk {
                id: shared.id,
                what: EventAskWhat::Skip,
            }));
            step.logs.push(skip_swallowed_line(shared.id));
        }
        SkipVerdict::Swallow
    }

    pub fn hear(
        &mut self,
        readings: &EventReadings,
        playing: &[PlayingHere],
        step: &mut EventStep,
    ) {
        let now_ms = readings.now_ms;
        for ask in &readings.asks {
            if ask.what == EventAskWhat::AlreadyPlayed {
                self.host_already_played(ask.id, now_ms, step);
            }
        }
        for id in &readings.started_heard {
            self.host_started(*id, now_ms);
            step.logs.push(host_start_heard_line(*id, "told"));
        }
        if let Some(id) = readings.partner_playing
            && !self.host_events.iter().any(|event| event.id == id)
        {
            self.host_started(id, now_ms);
            step.logs
                .push(host_start_heard_line(id, "seen in its state"));
        }
        for (id, skipped) in &readings.ended_heard {
            self.host_ended(*id, *skipped, now_ms);
            step.logs.push(host_end_heard_line(*id, *skipped));
            if mirrored_event_playing(playing, *id) {
                let skip_asked = self.skip_ask.is_some_and(|(asked, _)| asked == *id);
                self.after_the_hosts_end(*id, *skipped || skip_asked, now_ms, step);
            }
            if self.skip_ask.is_some_and(|(asked, _)| asked == *id) {
                self.skip_ask = None;
            }
        }
        self.host_events.retain(|event| match event.ended {
            Some((ended_at_ms, _)) => {
                now_ms.saturating_sub(ended_at_ms) <= HOST_EVENT_START_REMEMBERED_MS
            }
            None => now_ms.saturating_sub(event.started_at_ms) <= EVENT_SEEN_REMEMBERED_MS,
        });
    }

    pub fn step_in_rule(
        &mut self,
        readings: &EventReadings,
        playing: &[PlayingHere],
        runs: &mut RunsForThePartner,
        seen: &SeenEvents,
        played: &PlayedHere,
        starts_by_data_id: bool,
        step: &mut EventStep,
    ) {
        let now_ms = readings.now_ms;
        self.step_held(now_ms, step);
        if let Some((id, since_ms)) = self.skip_ask {
            let waited_ms = now_ms.saturating_sub(since_ms);
            if !mirrored_event_playing(playing, id) {
                self.skip_ask = None;
            } else if waited_ms >= GUEST_SKIP_ASK_GIVE_UP_MS {
                self.skip_ask = None;
                step.calls.push(EventCall::Skip);
                step.logs.push(skip_ask_gave_up_line(id, waited_ms));
            }
        }
        let (due, kept): (Vec<_>, Vec<_>) = self
            .outlasts
            .iter()
            .partition(|(_, skip_at_ms)| now_ms >= *skip_at_ms);
        self.outlasts = kept;
        for (id, _) in due {
            if mirrored_event_playing(playing, id) {
                step.calls.push(EventCall::Skip);
                step.logs.push(outlast_skip_line(id));
            }
        }
        if starts_by_data_id {
            self.follow_the_hosts_starts(readings, playing, runs, seen, played, step);
        }
    }

    pub fn release_all(&mut self, now_ms: u64, step: &mut EventStep) {
        if self.held.is_empty() && self.skip_ask.is_none() {
            self.outlasts.clear();
            return;
        }
        step.logs
            .push(link_lost_line(self.held.len(), self.skip_ask.is_some()));
        for held in self.held.drain(..) {
            step.calls.push(EventCall::ReleaseHeldPlay(held.id));
            step.logs.push(held_released_line(
                held.id,
                now_ms.saturating_sub(held.since_ms),
                "the link or the rule is down",
            ));
        }
        if self.skip_ask.take().is_some() {
            step.calls.push(EventCall::Skip);
        }
        self.outlasts.clear();
    }

    fn host_already_played(&mut self, id: EventId, now_ms: u64, step: &mut EventStep) {
        let held = self.holds(id);
        step.logs.push(already_played_heard_line(id, held));
        if !held {
            return;
        }
        let since_ms = self
            .held
            .iter()
            .find(|play| play.id == id)
            .map_or(now_ms, |play| play.since_ms);
        self.held.retain(|play| play.id != id);
        step.calls.push(EventCall::ReleaseHeldPlay(id));
        step.logs.push(held_released_line(
            id,
            now_ms.saturating_sub(since_ms),
            "the host already played it",
        ));
    }

    fn host_end_of(&self, id: EventId) -> Option<(u64, bool)> {
        self.host_events
            .iter()
            .find(|event| event.id == id)
            .and_then(|event| event.ended)
    }

    fn host_started(&mut self, id: EventId, now_ms: u64) {
        self.host_events.retain(|event| event.id != id);
        if self.host_events.len() >= MOST_HOST_EVENTS_REMEMBERED {
            self.host_events.remove(0);
        }
        self.host_events.push(HostEvent {
            id,
            started_at_ms: now_ms,
            ended: None,
            followed: false,
        });
    }

    fn host_ended(&mut self, id: EventId, skipped: bool, now_ms: u64) {
        if !self.host_events.iter().any(|event| event.id == id) {
            self.host_started(id, now_ms);
        }
        if let Some(event) = self.host_events.iter_mut().find(|event| event.id == id) {
            event.ended = Some((now_ms, skipped));
        }
    }

    fn after_the_hosts_end(
        &mut self,
        id: EventId,
        skipped: bool,
        now_ms: u64,
        step: &mut EventStep,
    ) {
        if skipped {
            self.outlasts.retain(|(outlasting, _)| *outlasting != id);
            step.calls.push(EventCall::Skip);
            step.logs.push(skipped_for_the_host_line(id));
        } else if !self
            .outlasts
            .iter()
            .any(|(outlasting, _)| *outlasting == id)
        {
            self.outlasts
                .push((id, now_ms + GUEST_EVENT_MAY_OUTLAST_THE_HOSTS_BY_MS));
            step.logs.push(outlast_started_line(id));
        }
    }

    fn step_held(&mut self, now_ms: u64, step: &mut EventStep) {
        let mut kept = Vec::with_capacity(self.held.len());
        for mut held in std::mem::take(&mut self.held) {
            let waited_ms = now_ms.saturating_sub(held.since_ms);
            if self.host_plays(held.id) || self.host_started_recently(held.id, now_ms) {
                // The released play starts here soon: never also start it by its DataId.
                for event in &mut self.host_events {
                    if event.id == held.id {
                        event.followed = true;
                    }
                }
                step.calls.push(EventCall::ReleaseHeldPlay(held.id));
                step.logs.push(held_released_line(
                    held.id,
                    waited_ms,
                    "the host started it",
                ));
                continue;
            }
            if waited_ms >= GUEST_EVENT_START_GIVE_UP_MS {
                step.calls.push(EventCall::ReleaseHeldPlay(held.id));
                step.logs.push(held_gave_up_line(held.id, waited_ms));
                continue;
            }
            let ask_due = held.asked_at_ms.is_none_or(|asked_at_ms| {
                now_ms.saturating_sub(asked_at_ms) >= EVENT_ASK_RESEND_MS
            });
            if ask_due {
                if held.asked_at_ms.is_some() {
                    step.logs.push(held_waiting_line(held.id, waited_ms));
                }
                held.asked_at_ms = Some(now_ms);
                step.sends.push(EventSend::Ask(EventAsk {
                    id: held.id,
                    what: EventAskWhat::Start,
                }));
            }
            kept.push(held);
        }
        self.held = kept;
    }

    fn follow_the_hosts_starts(
        &mut self,
        readings: &EventReadings,
        playing: &[PlayingHere],
        runs: &mut RunsForThePartner,
        seen: &SeenEvents,
        played: &PlayedHere,
        step: &mut EventStep,
    ) {
        let now_ms = readings.now_ms;
        let held: Vec<EventId> = self.held.iter().map(|held| held.id).collect();
        let mut wait_reason = None;
        for event in &mut self.host_events {
            let played_ms = now_ms.saturating_sub(event.started_at_ms);
            if event.followed || event.ended.is_some() || played_ms < EVENT_FOLLOW_GRACE_MS {
                continue;
            }
            let here_already = held.contains(&event.id)
                || playing.iter().any(|here| here.id == event.id)
                || runs.has(event.id)
                || played.contains(event.id)
                || seen.contains(event.id, &readings.current_place, now_ms);
            if here_already {
                event.followed = true;
                continue;
            }
            if !readings.in_the_partners_place() {
                wait_reason = Some((event.id, played_ms, "this game is at another place"));
            } else if readings.plays_an_event() || !runs.is_empty() {
                wait_reason = Some((event.id, played_ms, "this game plays another event"));
            } else {
                event.followed = true;
                runs.begin(event.id, now_ms, step);
                step.logs.push(follow_start_line(event.id, played_ms));
            }
        }
        if let Some((id, played_ms, reason)) = wait_reason {
            let due = self.follow_wait_logged_at_ms.is_none_or(|logged_at_ms| {
                now_ms.saturating_sub(logged_at_ms) >= EVENT_FOLLOW_WAIT_LOG_INTERVAL_MS
            });
            if due {
                self.follow_wait_logged_at_ms = Some(now_ms);
                step.logs.push(follow_waits_line(id, played_ms, reason));
            }
        }
    }
}
