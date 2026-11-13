use crate::load_gate::{FIRST_SYSTEM_PHASE_ID, PARTNER_LOAD_SIGNAL_STALE_MS, StoryPlace};
use crate::story_follow::request_to_make::request_to_make;
use crate::story_follow::{
    FollowReadings, FollowSend, FollowStep, GameRequestVerdict, MOST_MOVES_QUEUED_AT_THE_GUEST,
    RELEASED_REQUEST_TRIED_MOST_MS, REQUEST_IN_FLIGHT_MOST_MS, STORY_ASK_GIVE_UP_MS,
    STORY_ASK_RESEND_MS, STORY_PLACE_CATCH_UP_AFTER_MS, StoryAsk, StoryAskAnswer, StoryMove,
    StoryRequest, answer_heard_line, ask_kept_waiting_line, ask_sent_line, catch_up_line,
    gave_up_line, in_flight_expired_line, link_lost_line, made_here_line,
    move_dropped_as_already_here_line, move_heard_line, request_kept_line, request_replaced_line,
};

#[derive(Debug, Clone)]
struct KeptRequest {
    number: u32,
    at: StoryPlace,
    request: StoryRequest,
    since_ms: u64,
    last_sent_ms: Option<u64>,
}

#[derive(Debug, Clone)]
struct ReleasedRequest {
    request: StoryRequest,
    since_ms: u64,
}

#[derive(Debug, Clone)]
struct QueuedMove {
    story_move: StoryMove,
    heard_at_ms: u64,
}

#[derive(Debug, Clone)]
struct InFlight {
    target: StoryPlace,
    since_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct GuestSide {
    last_ask_number: u32,
    kept: Option<KeptRequest>,
    released: Option<ReleasedRequest>,
    queue: Vec<QueuedMove>,
    last_move_number_taken: u32,
    in_flight: Option<InFlight>,
    differing_since_ms: Option<u64>,
}

impl GuestSide {
    pub(super) fn game_request(
        &mut self,
        request: &StoryRequest,
        current: &StoryPlace,
        now_ms: u64,
        logs: &mut Vec<String>,
    ) -> GameRequestVerdict {
        let target = request.target_place();
        if target.has_a_name()
            && self
                .in_flight
                .as_ref()
                .is_some_and(|in_flight| in_flight.target == target)
        {
            return GameRequestVerdict::Swallow;
        }
        self.last_ask_number += 1;
        let number = self.last_ask_number;
        if let Some(old) = self.kept.take() {
            logs.push(request_replaced_line(old.number, number));
        }
        logs.push(request_kept_line(number, request));
        self.kept = Some(KeptRequest {
            number,
            at: *current,
            request: request.clone(),
            since_ms: now_ms,
            last_sent_ms: None,
        });
        GameRequestVerdict::Swallow
    }

    pub(super) fn step(
        &mut self,
        readings: &FollowReadings,
        can_take_a_request: bool,
        step: &mut FollowStep,
    ) {
        let now_ms = readings.now_ms;
        self.expire_the_request_in_flight(readings, step);
        for answer in &readings.answers {
            self.hear_answer(answer, step);
        }
        for story_move in &readings.moves {
            self.hear_move(story_move, now_ms, step);
        }
        self.give_up_a_silent_ask(now_ms, step);
        self.send_or_wait_on_the_kept_ask(now_ms, step);
        self.track_the_difference(readings);
        self.expire_a_released_request(now_ms);
        if can_take_a_request {
            self.make_one(readings, step);
        }
    }

    pub(super) fn step_alone(
        &mut self,
        readings: &FollowReadings,
        can_take_a_request: bool,
        step: &mut FollowStep,
    ) {
        self.expire_the_request_in_flight(readings, step);
        self.expire_a_released_request(readings.now_ms);
        self.differing_since_ms = None;
        if can_take_a_request {
            self.make_a_released_request(readings, step);
        }
    }

    pub(super) fn wanted(&self, readings: &FollowReadings) -> Option<StoryRequest> {
        if let Some(head) = self.queue.first() {
            return Some(head.story_move.request.clone());
        }
        if self.released.is_some() {
            return self.wanted_alone();
        }
        self.catch_up_request(readings).map(|(request, _)| request)
    }

    pub(super) fn wanted_alone(&self) -> Option<StoryRequest> {
        self.released
            .as_ref()
            .map(|released| released.request.clone())
    }

    pub(super) fn partner_gone(&mut self, now_ms: u64, step: &mut FollowStep) {
        step.logs
            .push(link_lost_line(self.kept.is_some(), self.queue.len()));
        if let Some(kept) = self.kept.take() {
            self.released = Some(ReleasedRequest {
                request: kept.request,
                since_ms: now_ms,
            });
        }
        self.queue.clear();
        self.last_move_number_taken = 0;
        self.differing_since_ms = None;
    }

    fn expire_the_request_in_flight(&mut self, readings: &FollowReadings, step: &mut FollowStep) {
        let Some(in_flight) = &self.in_flight else {
            return;
        };
        if readings.current_place == in_flight.target {
            self.in_flight = None;
        } else if readings.now_ms.saturating_sub(in_flight.since_ms) >= REQUEST_IN_FLIGHT_MOST_MS {
            step.logs.push(in_flight_expired_line(&in_flight.target));
            self.in_flight = None;
        }
    }

    fn expire_a_released_request(&mut self, now_ms: u64) {
        if self.released.as_ref().is_some_and(|released| {
            now_ms.saturating_sub(released.since_ms) >= RELEASED_REQUEST_TRIED_MOST_MS
        }) {
            self.released = None;
        }
    }

    fn hear_answer(&mut self, answer: &StoryAskAnswer, step: &mut FollowStep) {
        let Some(kept) = &self.kept else {
            return;
        };
        if kept.number != answer.number {
            return;
        }
        step.logs.push(answer_heard_line(answer, &kept.request));
        self.kept = None;
    }

    fn hear_move(&mut self, story_move: &StoryMove, now_ms: u64, step: &mut FollowStep) {
        let already_known = story_move.number <= self.last_move_number_taken
            || self
                .queue
                .iter()
                .any(|queued| queued.story_move.number == story_move.number);
        if already_known {
            return;
        }
        self.queue.push(QueuedMove {
            story_move: story_move.clone(),
            heard_at_ms: now_ms,
        });
        self.queue.sort_by_key(|queued| queued.story_move.number);
        if self.queue.len() > MOST_MOVES_QUEUED_AT_THE_GUEST {
            self.queue.remove(0);
        }
        step.logs
            .push(move_heard_line(story_move, self.queue.len()));
        self.kept = None;
    }

    fn give_up_a_silent_ask(&mut self, now_ms: u64, step: &mut FollowStep) {
        let Some(kept) = &self.kept else {
            return;
        };
        let waited_ms = now_ms.saturating_sub(kept.since_ms);
        if waited_ms < STORY_ASK_GIVE_UP_MS {
            return;
        }
        step.logs
            .push(gave_up_line(kept.number, waited_ms, &kept.request));
        self.released = Some(ReleasedRequest {
            request: kept.request.clone(),
            since_ms: now_ms,
        });
        self.kept = None;
    }

    fn send_or_wait_on_the_kept_ask(&mut self, now_ms: u64, step: &mut FollowStep) {
        let Some(kept) = &mut self.kept else {
            return;
        };
        let due = kept
            .last_sent_ms
            .is_none_or(|sent_ms| now_ms.saturating_sub(sent_ms) >= STORY_ASK_RESEND_MS);
        if !due {
            return;
        }
        let ask = StoryAsk {
            number: kept.number,
            at: kept.at,
            request: kept.request.clone(),
        };
        step.logs.push(if kept.last_sent_ms.is_none() {
            ask_sent_line(&ask)
        } else {
            ask_kept_waiting_line(
                kept.number,
                now_ms.saturating_sub(kept.since_ms),
                &kept.request,
            )
        });
        kept.last_sent_ms = Some(now_ms);
        step.sends.push(FollowSend::Ask(ask));
    }

    fn track_the_difference(&mut self, readings: &FollowReadings) {
        let differs = readings.host_place.has_a_name()
            && readings.current_place.has_a_name()
            && readings.host_place != readings.current_place;
        if differs {
            self.differing_since_ms.get_or_insert(readings.now_ms);
        } else {
            self.differing_since_ms = None;
        }
    }

    fn already_here(&self, request: &StoryRequest, current: &StoryPlace) -> bool {
        let target = request.target_place();
        target.has_a_name()
            && (target == *current
                || self
                    .in_flight
                    .as_ref()
                    .is_some_and(|in_flight| in_flight.target == target))
    }

    fn make_one(&mut self, readings: &FollowReadings, step: &mut FollowStep) {
        while let Some(head) = self.queue.first() {
            if !self.already_here(&head.story_move.request, &readings.current_place) {
                break;
            }
            step.logs.push(move_dropped_as_already_here_line(
                head.story_move.number,
                &head.story_move.request,
            ));
            self.last_move_number_taken = head.story_move.number;
            self.queue.remove(0);
        }
        if self.queue.is_empty() {
            if self.released.is_some() {
                self.make_a_released_request(readings, step);
            } else {
                self.catch_up(readings, step);
            }
            return;
        }
        let queued = self.queue.remove(0);
        self.last_move_number_taken = queued.story_move.number;
        let delay_ms = readings.now_ms.saturating_sub(queued.heard_at_ms);
        let why = format!("the host's move {}", queued.story_move.number);
        self.make(&queued.story_move.request, &why, delay_ms, readings, step);
    }

    fn make_a_released_request(&mut self, readings: &FollowReadings, step: &mut FollowStep) {
        let Some(released) = self.released.take() else {
            return;
        };
        if self.already_here(&released.request, &readings.current_place) {
            return;
        }
        let delay_ms = readings.now_ms.saturating_sub(released.since_ms);
        self.make(&released.request, "run here", delay_ms, readings, step);
    }

    fn catch_up(&mut self, readings: &FollowReadings, step: &mut FollowStep) {
        let Some((request, differing_ms)) = self.catch_up_request(readings) else {
            return;
        };
        let host_place = readings.host_place;
        let current = readings.current_place;
        step.logs
            .push(catch_up_line(&host_place, &current, differing_ms, &request));
        self.differing_since_ms = None;
        self.make(&request, "catch-up", differing_ms, readings, step);
    }

    fn catch_up_request(&self, readings: &FollowReadings) -> Option<(StoryRequest, u64)> {
        let host_place = readings.host_place;
        let current = readings.current_place;
        let places_are_story =
            host_place.phase_id < FIRST_SYSTEM_PHASE_ID && current.phase_id < FIRST_SYSTEM_PHASE_ID;
        let report_is_fresh = readings.host_report_heard_at_ms > 0
            && readings
                .now_ms
                .saturating_sub(readings.host_report_heard_at_ms)
                <= PARTNER_LOAD_SIGNAL_STALE_MS;
        let host_is_somewhere_real = readings.host_in_play || readings.host_waiting_at_load_end;
        let differing_ms = self
            .differing_since_ms
            .map(|since_ms| readings.now_ms.saturating_sub(since_ms))
            .filter(|differing_ms| *differing_ms >= STORY_PLACE_CATCH_UP_AFTER_MS)?;
        if !places_are_story
            || !report_is_fresh
            || !host_is_somewhere_real
            || self.in_flight.is_some()
            || self.kept.is_some()
        {
            return None;
        }
        let request = if host_place.phase_id == current.phase_id {
            StoryRequest::SubPhase {
                id: host_place.phase_id,
                name: host_place.sub_phase(),
            }
        } else {
            StoryRequest::full_with_default_params(host_place.phase_id, &host_place.sub_phase())
        };
        Some((request, differing_ms))
    }

    fn make(
        &mut self,
        request: &StoryRequest,
        why: &str,
        delay_ms: u64,
        readings: &FollowReadings,
        step: &mut FollowStep,
    ) {
        let to_make = request_to_make(request, &readings.current_place);
        step.logs.push(made_here_line(why, &to_make, delay_ms));
        self.in_flight = Some(InFlight {
            target: to_make.target_place(),
            since_ms: readings.now_ms,
        });
        step.make = Some(to_make);
    }
}
