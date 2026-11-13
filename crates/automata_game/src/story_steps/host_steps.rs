use std::collections::{BTreeSet, VecDeque};

use crate::load_gate::StoryPlace;
use crate::story_steps::step_lines::{ask_answered_line, kept_done_line};
use crate::story_steps::{
    ActionLook, MOST_TOLD_STEPS_KEPT_AT_THE_HOST, SetDone, SetDoneWhy, StepActionState, StepAnswer,
    StepSend, StepVerdict, StepsStep, StoryStepAnswer, StoryStepAsk, StoryStepDone,
};

/// The host's side: tells its dones in order and answers the guest's asks.
#[derive(Debug, Clone, Default)]
pub(crate) struct HostSteps {
    last_number: u32,
    told: VecDeque<StoryStepDone>,
    to_tell: Vec<StoryStepDone>,
    kept_done: BTreeSet<u32>,
    kept_done_logged: BTreeSet<u32>,
}

impl HostSteps {
    pub(crate) fn own_done(&mut self, id: u32, value: i32, place: &StoryPlace) {
        self.last_number += 1;
        let done = StoryStepDone {
            number: self.last_number,
            id,
            value,
            place: *place,
        };
        self.told.retain(|told| told.id != id);
        if self.told.len() >= MOST_TOLD_STEPS_KEPT_AT_THE_HOST {
            self.told.pop_front();
        }
        self.told.push_back(done.clone());
        self.to_tell.push(done);
    }

    pub(crate) fn leave_done(&mut self, id: u32, logs: &mut Vec<String>) -> StepVerdict {
        if !self.kept_done.contains(&id) {
            return StepVerdict::Pass;
        }
        if self.kept_done_logged.insert(id) {
            logs.push(kept_done_line(id));
        }
        StepVerdict::Refuse
    }

    pub(crate) fn step(
        &mut self,
        answers_asks: bool,
        place: &StoryPlace,
        asks: &[StoryStepAsk],
        looks: &[ActionLook],
        step: &mut StepsStep,
    ) {
        if answers_asks {
            for ask in asks {
                self.answer(ask, place, looks, step);
            }
        }
        step.sends
            .extend(self.to_tell.drain(..).map(StepSend::Done));
    }

    pub(crate) fn forget(&mut self, id: u32) -> bool {
        let told_before = self.told.len();
        self.told.retain(|told| told.id != id);
        let was_kept = self.kept_done.remove(&id);
        was_kept || self.told.len() != told_before
    }

    pub(crate) fn has_lists(&self) -> bool {
        !self.told.is_empty() || !self.kept_done.is_empty()
    }

    pub(crate) fn clear_lists(&mut self) {
        self.told.clear();
        self.kept_done.clear();
        self.kept_done_logged.clear();
    }

    fn answer(
        &mut self,
        ask: &StoryStepAsk,
        place: &StoryPlace,
        looks: &[ActionLook],
        step: &mut StepsStep,
    ) {
        let answer_with = |answer: StepAnswer, step: &mut StepsStep| {
            step.sends
                .push(StepSend::Answer(StoryStepAnswer { id: ask.id, answer }));
        };
        if ask.place != *place {
            step.logs
                .push(ask_answered_line(ask, "the guest is at another place"));
            answer_with(StepAnswer::OtherPlace, step);
            return;
        }
        if let Some(told) = self.told.iter().find(|told| told.id == ask.id) {
            step.logs
                .push(ask_answered_line(ask, "done here, told again"));
            step.sends.push(StepSend::Done(told.clone()));
            return;
        }
        let look = looks.iter().find(|look| look.id == ask.id);
        match look.and_then(|look| look.state) {
            Some(StepActionState::Done) => {
                let value = look.map_or(ask.value, |look| look.value);
                step.logs.push(ask_answered_line(ask, "done here, told"));
                self.own_done(ask.id, value, place);
            }
            Some(StepActionState::Running) => {
                step.logs
                    .push(ask_answered_line(ask, "it runs here, set done here"));
                self.kept_done.insert(ask.id);
                step.set_done.push(SetDone {
                    id: ask.id,
                    value: ask.value,
                    why: SetDoneWhy::AskedByTheGuest,
                });
            }
            Some(StepActionState::NotStarted) | None => {
                step.logs.push(ask_answered_line(ask, "not started here"));
                answer_with(StepAnswer::NotStarted, step);
            }
        }
    }
}
