use std::collections::BTreeSet;

use crate::load_gate::StoryPlace;
use crate::story_steps::step_lines::{
    already_done_here_line, answer_heard_line, gave_up_line, held_line, holding_line,
    host_done_heard_line, kept_done_line, link_lost_line, refused_retries_line, too_many_held_line,
    watching_long_wait_line,
};
use crate::story_steps::{
    ActionLook, HELD_STEPS_WAIT_LINE_INTERVAL_MS, MOST_HELD_STORY_STEPS, MOST_PENDING_HOST_STEPS,
    ObservedDone, STORY_STEP_ASK_RESEND_MS, STORY_STEP_GIVE_UP_MS,
    STORY_STEP_HOST_NOT_STARTED_WAIT_MS, STORY_STEP_LONG_WAIT_LOG_MS,
    STORY_STEP_REFUSED_RETRIES_LOG_MS, SetDone, SetDoneWhy, StepActionState, StepSend, StepVerdict,
    StepsStep, StoryStepAnswer, StoryStepAsk, StoryStepDone,
};

#[derive(Debug, Clone, Copy)]
struct HeldStep {
    id: u32,
    class: &'static str,
    own_value: i32,
    held_at_ms: u64,
    last_ask_at_ms: Option<u64>,
    not_started_answer_at_ms: Option<u64>,
    refused_retries: u32,
    last_retries_line_at_ms: u64,
    last_long_wait_line_at_ms: u64,
}

/// The guest's side in mode `on`: holds its own dones, asks, applies the host's.
#[derive(Debug, Clone, Default)]
pub(crate) struct GuestSteps {
    held: Vec<HeldStep>,
    pending: Vec<StoryStepDone>,
    kept_done: BTreeSet<u32>,
    kept_done_logged: BTreeSet<u32>,
    passed: BTreeSet<u32>,
    last_holding_line_at_ms: Option<u64>,
}

impl GuestSteps {
    pub(crate) fn own_done(
        &mut self,
        done: &ObservedDone,
        place: &StoryPlace,
        now_ms: u64,
        logs: &mut Vec<String>,
    ) -> StepVerdict {
        let id = done.id;
        if done.ours {
            if !self.passed.contains(&id) {
                self.take_as_done(id);
            }
            return StepVerdict::Pass;
        }
        if self.kept_done.contains(&id) || self.passed.contains(&id) {
            return StepVerdict::Pass;
        }
        if self.pending.iter().any(|pending| pending.id == id) {
            self.take_as_done(id);
            return StepVerdict::Pass;
        }
        if let Some(held) = self.held.iter_mut().find(|held| held.id == id) {
            held.refused_retries += 1;
            return StepVerdict::Refuse;
        }
        if self.held.len() >= MOST_HELD_STORY_STEPS {
            logs.push(too_many_held_line(id));
            return StepVerdict::Pass;
        }
        logs.push(held_line(done, place, now_ms));
        self.held.push(HeldStep {
            id,
            class: done.class_name(),
            own_value: done.value,
            held_at_ms: now_ms,
            last_ask_at_ms: None,
            not_started_answer_at_ms: None,
            refused_retries: 0,
            last_retries_line_at_ms: now_ms,
            last_long_wait_line_at_ms: now_ms,
        });
        StepVerdict::Refuse
    }

    pub(crate) fn is_holding(&self, id: u32) -> bool {
        self.held.iter().any(|held| held.id == id)
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

    pub(crate) fn ids_to_look_at(&self) -> Vec<u32> {
        self.pending.iter().map(|pending| pending.id).collect()
    }

    pub(crate) fn hear_done(&mut self, done: &StoryStepDone, now_ms: u64, logs: &mut Vec<String>) {
        if self.kept_done.contains(&done.id) {
            return;
        }
        let held_ms = self
            .held
            .iter()
            .position(|held| held.id == done.id)
            .map(|index| now_ms.saturating_sub(self.held.remove(index).held_at_ms));
        logs.push(host_done_heard_line(done, held_ms));
        self.passed.remove(&done.id);
        self.pending.retain(|pending| pending.id != done.id);
        if self.pending.len() >= MOST_PENDING_HOST_STEPS {
            self.pending.remove(0);
        }
        self.pending.push(done.clone());
    }

    pub(crate) fn hear_answer(
        &mut self,
        answer: &StoryStepAnswer,
        watching: bool,
        now_ms: u64,
        logs: &mut Vec<String>,
    ) {
        let Some(held) = self.held.iter_mut().find(|held| held.id == answer.id) else {
            return;
        };
        if held.not_started_answer_at_ms.is_none() {
            held.not_started_answer_at_ms = Some(now_ms);
            logs.push(answer_heard_line(answer.id, answer.answer, watching));
        }
    }

    pub(crate) fn step(
        &mut self,
        place: &StoryPlace,
        watching: bool,
        now_ms: u64,
        looks: &[ActionLook],
        step: &mut StepsStep,
    ) {
        self.apply_pending(looks, step);
        self.give_up_or_ask(place, watching, now_ms, step);
        if let Some(oldest) = self.held.iter().min_by_key(|held| held.held_at_ms)
            && self
                .last_holding_line_at_ms
                .is_none_or(|at| now_ms.saturating_sub(at) >= HELD_STEPS_WAIT_LINE_INTERVAL_MS)
        {
            self.last_holding_line_at_ms = Some(now_ms);
            step.logs.push(holding_line(
                self.held.len(),
                oldest.id,
                now_ms.saturating_sub(oldest.held_at_ms),
            ));
        }
    }

    pub(crate) fn link_lost(&mut self, step: &mut StepsStep) {
        if !self.held.is_empty() {
            step.logs.push(link_lost_line(self.held.len()));
        }
        for held in self.held.drain(..) {
            self.passed.insert(held.id);
            step.set_done.push(SetDone {
                id: held.id,
                value: held.own_value,
                why: SetDoneWhy::LinkLost,
            });
        }
        self.pending.clear();
    }

    pub(crate) fn forget(&mut self, id: u32) -> bool {
        let before = self.held.len() + self.pending.len();
        self.held.retain(|held| held.id != id);
        self.pending.retain(|pending| pending.id != id);
        self.kept_done_logged.remove(&id);
        let was_kept = self.kept_done.remove(&id);
        let had_passed = self.passed.remove(&id);
        was_kept || had_passed || self.held.len() + self.pending.len() != before
    }

    pub(crate) fn has_lists(&self) -> bool {
        !self.held.is_empty()
            || !self.pending.is_empty()
            || !self.kept_done.is_empty()
            || !self.passed.is_empty()
    }

    pub(crate) fn clear_lists(&mut self) {
        *self = Self::default();
    }

    fn take_as_done(&mut self, id: u32) {
        self.held.retain(|held| held.id != id);
        self.pending.retain(|pending| pending.id != id);
        self.kept_done.insert(id);
    }

    fn apply_pending(&mut self, looks: &[ActionLook], step: &mut StepsStep) {
        let mut still_pending = Vec::with_capacity(self.pending.len());
        for done in std::mem::take(&mut self.pending) {
            let state = looks
                .iter()
                .find(|look| look.id == done.id)
                .and_then(|look| look.state);
            match state {
                Some(StepActionState::Running) => {
                    self.kept_done.insert(done.id);
                    step.set_done.push(SetDone {
                        id: done.id,
                        value: done.value,
                        why: SetDoneWhy::HostsDone,
                    });
                }
                Some(StepActionState::Done) => {
                    self.kept_done.insert(done.id);
                    step.logs.push(already_done_here_line(done.id));
                }
                Some(StepActionState::NotStarted) | None => still_pending.push(done),
            }
        }
        self.pending = still_pending;
    }

    fn give_up_or_ask(
        &mut self,
        place: &StoryPlace,
        watching: bool,
        now_ms: u64,
        step: &mut StepsStep,
    ) {
        let mut kept = Vec::with_capacity(self.held.len());
        for mut held in std::mem::take(&mut self.held) {
            let waited_ms = now_ms.saturating_sub(held.held_at_ms);
            if held.refused_retries > 0
                && now_ms.saturating_sub(held.last_retries_line_at_ms)
                    >= STORY_STEP_REFUSED_RETRIES_LOG_MS
            {
                held.last_retries_line_at_ms = now_ms;
                step.logs
                    .push(refused_retries_line(held.id, held.refused_retries));
            }
            if watching
                && now_ms.saturating_sub(held.last_long_wait_line_at_ms)
                    >= STORY_STEP_LONG_WAIT_LOG_MS
            {
                held.last_long_wait_line_at_ms = now_ms;
                step.logs.push(watching_long_wait_line(
                    held.id, held.class, place, waited_ms,
                ));
            }
            let gives_up = !watching
                && match held.not_started_answer_at_ms {
                    Some(at) => now_ms.saturating_sub(at) >= STORY_STEP_HOST_NOT_STARTED_WAIT_MS,
                    None => waited_ms >= STORY_STEP_GIVE_UP_MS,
                };
            if gives_up {
                step.logs.push(gave_up_line(
                    held.id,
                    waited_ms,
                    held.not_started_answer_at_ms.is_some(),
                ));
                self.passed.insert(held.id);
                step.set_done.push(SetDone {
                    id: held.id,
                    value: held.own_value,
                    why: SetDoneWhy::GaveUpWaiting,
                });
                continue;
            }
            if held
                .last_ask_at_ms
                .is_none_or(|at| now_ms.saturating_sub(at) >= STORY_STEP_ASK_RESEND_MS)
            {
                held.last_ask_at_ms = Some(now_ms);
                step.sends.push(StepSend::Ask(StoryStepAsk {
                    id: held.id,
                    value: held.own_value,
                    place: *place,
                }));
            }
            kept.push(held);
        }
        self.held = kept;
    }
}
