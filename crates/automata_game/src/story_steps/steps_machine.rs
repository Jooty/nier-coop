use std::collections::BTreeSet;

use crate::load_gate::StoryPlace;
use crate::story_steps::guest_steps::GuestSteps;
use crate::story_steps::guest_watch::GuestWatch;
use crate::story_steps::host_steps::HostSteps;
use crate::story_steps::step_lines::{
    lists_emptied_line, mirrored_done_line, places_differ_line, reset_forgotten_line,
    told_other_place_line, unknown_class_done_line, unmirrored_done_line,
    unmirrored_lines_end_line,
};
use crate::story_steps::{
    ActionLook, MOST_PENDING_HOST_STEPS, MOST_UNKNOWN_CLASSES_NAMED, MOST_UNMIRRORED_DONE_LINES,
    ObservedDone, StepVerdict, StepsStep, StoryStepAnswer, StoryStepAsk, StoryStepDone,
    StoryStepsMode,
};
use crate::travel::JumpRole;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StepsReadings {
    pub now_ms: u64,
    pub role: JumpRole,
    pub partner_present: bool,
    pub place: StoryPlace,
    pub partner_place: StoryPlace,
    pub told: Vec<StoryStepDone>,
    pub asks: Vec<StoryStepAsk>,
    pub answers: Vec<StoryStepAnswer>,
    pub looks: Vec<ActionLook>,
}

/// The host's story steps are the guest's: dones of mirrored story actions, by action id.
#[derive(Debug, Clone)]
pub struct StepsMachine {
    mode: StoryStepsMode,
    role: JumpRole,
    place: StoryPlace,
    partner_place: StoryPlace,
    host: HostSteps,
    guest: GuestSteps,
    watch: GuestWatch,
    told_for_another_place: Vec<StoryStepDone>,
    unmirrored_lines_left: u32,
    unknown_classes_named: BTreeSet<u32>,
    logs_before_the_next_step: Vec<String>,
}

impl StepsMachine {
    pub fn new(mode: StoryStepsMode) -> Self {
        Self {
            mode,
            role: JumpRole::Solo,
            place: StoryPlace::default(),
            partner_place: StoryPlace::default(),
            host: HostSteps::default(),
            guest: GuestSteps::default(),
            watch: GuestWatch::default(),
            told_for_another_place: Vec::new(),
            unmirrored_lines_left: MOST_UNMIRRORED_DONE_LINES,
            unknown_classes_named: BTreeSet::new(),
            logs_before_the_next_step: Vec::new(),
        }
    }

    pub fn mode(&self) -> StoryStepsMode {
        self.mode
    }

    pub fn role(&self) -> JumpRole {
        self.role
    }

    /// True while the detours must look at state changes: a mode that hooks and a partner.
    pub fn is_acting(&self) -> bool {
        self.mode.installs_hooks() && self.role != JumpRole::Solo
    }

    pub fn on_own_done(&mut self, done: &ObservedDone, now_ms: u64) -> StepVerdict {
        if !self.is_acting() {
            return StepVerdict::Pass;
        }
        if !done.is_mirrored() {
            self.note_unmirrored(done, now_ms);
            return StepVerdict::Pass;
        }
        let logs = &mut self.logs_before_the_next_step;
        logs.push(mirrored_done_line(self.role, done, &self.place, now_ms));
        match self.role {
            JumpRole::Solo => StepVerdict::Pass,
            JumpRole::Host => {
                self.host.own_done(done.id, done.value, &self.place);
                StepVerdict::Pass
            }
            JumpRole::Guest if !self.mode.holds_and_applies() => {
                self.watch.own_done(done, now_ms, logs);
                StepVerdict::Pass
            }
            JumpRole::Guest => {
                if !done.ours && self.place != self.partner_place {
                    logs.push(places_differ_line(
                        done.id,
                        &self.place,
                        &self.partner_place,
                    ));
                    return StepVerdict::Pass;
                }
                self.guest.own_done(done, &self.place, now_ms, logs)
            }
        }
    }

    /// A change that takes a done blocking story action out of done.
    pub fn on_leave_done(&mut self, id: u32) -> StepVerdict {
        if !self.is_acting() || !self.mode.holds_and_applies() {
            return StepVerdict::Pass;
        }
        let logs = &mut self.logs_before_the_next_step;
        match self.role {
            JumpRole::Solo => StepVerdict::Pass,
            JumpRole::Host => self.host.leave_done(id, logs),
            JumpRole::Guest => self.guest.leave_done(id, logs),
        }
    }

    pub fn on_reset(&mut self, id: u32) {
        if !self.is_acting() {
            return;
        }
        let in_host_lists = self.host.forget(id);
        let in_guest_lists = self.guest.forget(id);
        let in_watch_lists = self.watch.forget(id);
        if in_host_lists || in_guest_lists || in_watch_lists {
            self.logs_before_the_next_step
                .push(reset_forgotten_line(id));
        }
    }

    /// The ids whose action state the next step needs (besides the ids of the dones and asks heard).
    pub fn ids_to_look_at(&self) -> Vec<u32> {
        if self.role == JumpRole::Guest && self.mode.holds_and_applies() {
            self.guest.ids_to_look_at()
        } else {
            Vec::new()
        }
    }

    pub fn step(&mut self, readings: &StepsReadings) -> StepsStep {
        let role = if readings.partner_present {
            readings.role
        } else {
            JumpRole::Solo
        };
        let mut step = StepsStep {
            logs: std::mem::take(&mut self.logs_before_the_next_step),
            ..StepsStep::default()
        };
        if role != self.role {
            self.change_role(role, &mut step);
        }
        self.partner_place = readings.partner_place;
        let mut told: Vec<StoryStepDone> = Vec::new();
        if readings.place != self.place {
            if self.has_lists() {
                step.logs
                    .push(lists_emptied_line("the phase or sub-phase changed"));
            }
            self.clear_lists();
            self.place = readings.place;
            told.extend(
                std::mem::take(&mut self.told_for_another_place)
                    .into_iter()
                    .filter(|done| done.place == self.place),
            );
        }
        if !self.is_acting() {
            return step;
        }
        told.extend(readings.told.iter().cloned());
        let now_ms = readings.now_ms;
        let holds = self.mode.holds_and_applies();
        match role {
            JumpRole::Solo => {}
            JumpRole::Host => {
                self.host.step(
                    holds,
                    &self.place,
                    &readings.asks,
                    &readings.looks,
                    &mut step,
                );
            }
            JumpRole::Guest => {
                for done in &told {
                    if done.place != self.place {
                        step.logs.push(told_other_place_line(done, &self.place));
                        self.keep_for_its_place(done);
                    } else if holds {
                        self.guest.hear_done(done, now_ms, &mut step.logs);
                    } else {
                        self.watch.hear_done(done, now_ms, &mut step.logs);
                    }
                }
                if holds {
                    for answer in &readings.answers {
                        self.guest.hear_answer(answer, now_ms, &mut step.logs);
                    }
                    self.guest
                        .step(&self.place, now_ms, &readings.looks, &mut step);
                } else {
                    self.watch.step(&self.place, now_ms, &mut step.logs);
                }
            }
        }
        step
    }

    fn change_role(&mut self, role: JumpRole, step: &mut StepsStep) {
        if self.role == JumpRole::Guest && self.mode.holds_and_applies() {
            self.guest.link_lost(step);
        }
        if self.has_lists() {
            step.logs.push(lists_emptied_line("the link changed"));
        }
        self.clear_lists();
        self.told_for_another_place.clear();
        self.role = role;
    }

    fn has_lists(&self) -> bool {
        self.host.has_lists() || self.guest.has_lists() || self.watch.has_lists()
    }

    fn clear_lists(&mut self) {
        self.host.clear_lists();
        self.guest.clear_lists();
        self.watch.clear_lists();
    }

    fn keep_for_its_place(&mut self, done: &StoryStepDone) {
        self.told_for_another_place
            .retain(|kept| kept.id != done.id);
        if self.told_for_another_place.len() >= MOST_PENDING_HOST_STEPS {
            self.told_for_another_place.remove(0);
        }
        self.told_for_another_place.push(done.clone());
    }

    fn note_unmirrored(&mut self, done: &ObservedDone, now_ms: u64) {
        let logs = &mut self.logs_before_the_next_step;
        if done.class.is_none() {
            if self.unknown_classes_named.len() < MOST_UNKNOWN_CLASSES_NAMED
                && self.unknown_classes_named.insert(done.vftable_rva)
            {
                logs.push(unknown_class_done_line(
                    self.role,
                    done,
                    &self.place,
                    now_ms,
                ));
            }
            return;
        }
        if self.unmirrored_lines_left == 0 {
            return;
        }
        self.unmirrored_lines_left -= 1;
        logs.push(unmirrored_done_line(self.role, done, &self.place, now_ms));
        if self.unmirrored_lines_left == 0 {
            logs.push(unmirrored_lines_end_line());
        }
    }
}
