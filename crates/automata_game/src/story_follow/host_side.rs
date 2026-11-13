use crate::load_gate::{PlaceRelation, StoryPlace};
use crate::story_follow::request_to_make::request_to_make;
use crate::story_follow::{
    AskVerdict, FollowSend, FollowStep, HOST_KEEPS_AN_ASK_WHILE_BUSY_MS,
    MOST_ANSWERED_ASKS_REMEMBERED, MOST_ASKS_KEPT_AT_THE_HOST, StoryAsk, StoryAskAnswer, StoryMove,
    StoryRequest, host_ask_answered_line, host_ask_made_line, move_told_line,
};

#[derive(Debug, Clone)]
struct KeptAsk {
    ask: StoryAsk,
    since_ms: u64,
}

#[derive(Debug, Clone)]
pub(super) struct HostSide {
    next_move_number: u32,
    last_move_target: Option<StoryPlace>,
    outbox: Vec<StoryMove>,
    kept: Vec<KeptAsk>,
    answered: Vec<StoryAskAnswer>,
}

impl Default for HostSide {
    fn default() -> Self {
        Self {
            next_move_number: 1,
            last_move_target: None,
            outbox: Vec::new(),
            kept: Vec::new(),
            answered: Vec::new(),
        }
    }
}

impl HostSide {
    pub(super) fn game_request(
        &mut self,
        request: &StoryRequest,
        current: &StoryPlace,
        logs: &mut Vec<String>,
    ) {
        let story_move = self.new_move(current, request.clone());
        logs.push(move_told_line(&story_move));
        self.outbox.push(story_move);
    }

    pub(super) fn step(
        &mut self,
        now_ms: u64,
        can_take_a_request: bool,
        current: &StoryPlace,
        asks: &[StoryAsk],
        step: &mut FollowStep,
    ) {
        step.sends
            .extend(self.outbox.drain(..).map(FollowSend::Move));
        for ask in asks {
            self.hear_ask(ask, now_ms, step);
        }
        let mut made_this_step = false;
        let kept = std::mem::take(&mut self.kept);
        for held in kept {
            let verdict = self.verdict_for(&held, now_ms, can_take_a_request, current);
            match verdict {
                Some(AskVerdict::Made) if made_this_step => self.kept.push(held),
                Some(AskVerdict::Made) => {
                    made_this_step = true;
                    self.make_for(&held.ask, current, step);
                    self.answer(held.ask.number, AskVerdict::Made, step);
                }
                Some(verdict) => self.answer(held.ask.number, verdict, step),
                None => self.kept.push(held),
            }
        }
    }

    pub(super) fn wanted(&self, now_ms: u64, current: &StoryPlace) -> Option<StoryRequest> {
        self.kept
            .iter()
            .find(|held| self.verdict_for(held, now_ms, true, current) == Some(AskVerdict::Made))
            .map(|held| request_to_make(&held.ask.request, current))
    }

    fn new_move(&mut self, current: &StoryPlace, request: StoryRequest) -> StoryMove {
        let story_move = StoryMove {
            number: self.next_move_number,
            from: *current,
            request,
        };
        self.next_move_number += 1;
        self.last_move_target = Some(story_move.request.target_place());
        story_move
    }

    fn hear_ask(&mut self, ask: &StoryAsk, now_ms: u64, step: &mut FollowStep) {
        if let Some(earlier) = self
            .answered
            .iter()
            .find(|answer| answer.number == ask.number)
        {
            step.sends.push(FollowSend::Answer(*earlier));
            return;
        }
        if self.kept.iter().any(|held| held.ask.number == ask.number) {
            return;
        }
        if self.kept.len() >= MOST_ASKS_KEPT_AT_THE_HOST {
            self.answer(ask.number, AskVerdict::RefusedBusyTooLong, step);
            return;
        }
        self.kept.push(KeptAsk {
            ask: ask.clone(),
            since_ms: now_ms,
        });
    }

    fn verdict_for(
        &self,
        held: &KeptAsk,
        now_ms: u64,
        can_take_a_request: bool,
        current: &StoryPlace,
    ) -> Option<AskVerdict> {
        let target = held.ask.request.target_place();
        let target_is_here_or_told =
            target.has_a_name() && (target == *current || self.last_move_target == Some(target));
        if target_is_here_or_told {
            Some(AskVerdict::AlreadyMade)
        } else if current.relation_to(&held.ask.at) != PlaceRelation::Same {
            Some(AskVerdict::RefusedOtherPlace)
        } else if can_take_a_request {
            Some(AskVerdict::Made)
        } else if now_ms.saturating_sub(held.since_ms) >= HOST_KEEPS_AN_ASK_WHILE_BUSY_MS {
            Some(AskVerdict::RefusedBusyTooLong)
        } else {
            None
        }
    }

    fn make_for(&mut self, ask: &StoryAsk, current: &StoryPlace, step: &mut FollowStep) {
        let request = request_to_make(&ask.request, current);
        step.logs.push(host_ask_made_line(ask.number, &request));
        let story_move = self.new_move(current, request.clone());
        step.logs.push(move_told_line(&story_move));
        step.sends.push(FollowSend::Move(story_move));
        step.make = Some(request);
    }

    fn answer(&mut self, number: u32, verdict: AskVerdict, step: &mut FollowStep) {
        let answer = StoryAskAnswer { number, verdict };
        step.logs.push(host_ask_answered_line(number, verdict));
        step.sends.push(FollowSend::Answer(answer));
        self.answered.push(answer);
        if self.answered.len() > MOST_ANSWERED_ASKS_REMEMBERED {
            self.answered.remove(0);
        }
    }
}
