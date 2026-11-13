use crate::cutscene_skip::{
    CutsceneSkipTracker, PARTNER_ASK_LIFETIME_MS, PromptAnswer, SKIP_PROMPT_VOTE_GIVES_UP_AFTER_MS,
    SkipPrompt, SkipVerdict, skip_prompt, skip_prompt_answer,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopVerdict {
    Run,
    Drop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteEvent {
    AskedAndHeld,
    PartnerAsks,
    AnsweredForAbsentPlayer,
    SkipAgreed,
    SkipAlone,
    DeclinedHere,
    DeclinedByPartner,
    GaveUp,
    HostAnswerRan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoteView {
    pub host: bool,
    pub partner_present: bool,
    pub unattended: bool,
    pub page_shown_here: bool,
    pub dialog_free: bool,
    pub now_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SentAnswer {
    pub prompt: u16,
    pub skip: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VoteStep {
    pub send: Option<SentAnswer>,
    pub fire: Option<&'static str>,
    pub open_page: Option<u16>,
    pub close_page: bool,
    pub event: Option<VoteEvent>,
}

/// The scene skip vote (`CutsceneSkipTracker`) run for a game prompt whose answer skips a
/// sequence both players are in. Only the host fires the agreed answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkipPromptVote {
    prompt: Option<SkipPrompt>,
    tracker: CutsceneSkipTracker,
    opened_ms: u64,
    local_answer: Option<PromptAnswer>,
    partner_declined: bool,
    page_opened_here: bool,
    release: Option<&'static str>,
}

impl SkipPromptVote {
    pub const fn new() -> Self {
        Self {
            prompt: None,
            tracker: CutsceneSkipTracker::new(),
            opened_ms: 0,
            local_answer: None,
            partner_declined: false,
            page_opened_here: false,
            release: None,
        }
    }

    pub fn open_prompt(&self) -> Option<SkipPrompt> {
        self.prompt
    }

    pub fn local_pop(&mut self, callback: &str, view: VoteView) -> (PopVerdict, VoteStep) {
        let Some((prompt, answer)) = skip_prompt_answer(callback) else {
            return (PopVerdict::Run, VoteStep::default());
        };
        if self.release == Some(prompt.callback_of(answer)) {
            self.release = None;
            return (PopVerdict::Run, VoteStep::default());
        }
        if !view.partner_present {
            let close_page = self.close();
            let event = (answer == PromptAnswer::Skip).then_some(VoteEvent::SkipAlone);
            return (
                PopVerdict::Run,
                VoteStep {
                    close_page,
                    event,
                    ..VoteStep::default()
                },
            );
        }
        self.open(prompt, view.now_ms);
        if view.host {
            self.host_answered(prompt, answer, view.now_ms)
        } else {
            (
                PopVerdict::Drop,
                self.guest_answered(prompt, answer, view.now_ms),
            )
        }
    }

    pub fn partner_answer(&mut self, prompt: u16, skip: bool, view: VoteView) -> VoteStep {
        let Some(prompt) = skip_prompt(prompt) else {
            return VoteStep::default();
        };
        if !view.partner_present {
            return VoteStep::default();
        }
        if skip {
            self.open(prompt, view.now_ms);
            let newly = self.tracker.partner_asked(view.now_ms);
            return VoteStep {
                event: newly.then_some(VoteEvent::PartnerAsks),
                ..VoteStep::default()
            };
        }
        if view.host {
            self.open(prompt, view.now_ms);
            self.partner_declined = true;
            if self.local_answer == Some(PromptAnswer::Skip) {
                return self.fire(PromptAnswer::Other, VoteEvent::DeclinedByPartner, false);
            }
            return VoteStep {
                event: Some(VoteEvent::DeclinedByPartner),
                ..VoteStep::default()
            };
        }
        if !self.is_open_for(prompt.prompt) {
            return VoteStep::default();
        }
        VoteStep {
            close_page: self.close(),
            event: Some(VoteEvent::DeclinedByPartner),
            ..VoteStep::default()
        }
    }

    pub fn host_answer_ran(&mut self, callback: &str) -> VoteStep {
        let Some((prompt, _)) = skip_prompt_answer(callback) else {
            return VoteStep::default();
        };
        if !self.is_open_for(prompt.prompt) {
            return VoteStep::default();
        }
        VoteStep {
            close_page: self.close(),
            event: Some(VoteEvent::HostAnswerRan),
            ..VoteStep::default()
        }
    }

    pub fn tick(&mut self, view: VoteView) -> VoteStep {
        let Some(prompt) = self.prompt else {
            return VoteStep::default();
        };
        if !view.partner_present {
            if view.host && self.local_answer == Some(PromptAnswer::Skip) {
                return self.fire(PromptAnswer::Skip, VoteEvent::SkipAlone, false);
            }
            return VoteStep {
                close_page: self.close(),
                ..VoteStep::default()
            };
        }
        let mut step = if view.host {
            self.host_tick(view)
        } else {
            self.guest_tick(prompt, view)
        };
        if step.send.is_none() && self.prompt.is_some() && self.tracker.resend_due(view.now_ms) {
            step.send = Some(SentAnswer {
                prompt: prompt.prompt,
                skip: true,
            });
        }
        step
    }

    fn host_answered(
        &mut self,
        prompt: SkipPrompt,
        answer: PromptAnswer,
        now_ms: u64,
    ) -> (PopVerdict, VoteStep) {
        if answer == PromptAnswer::Other {
            self.close();
            return (
                PopVerdict::Run,
                VoteStep {
                    send: Some(SentAnswer {
                        prompt: prompt.prompt,
                        skip: false,
                    }),
                    event: Some(VoteEvent::DeclinedHere),
                    ..VoteStep::default()
                },
            );
        }
        if self.partner_declined {
            return (
                PopVerdict::Drop,
                self.fire(PromptAnswer::Other, VoteEvent::DeclinedByPartner, false),
            );
        }
        self.local_answer = Some(PromptAnswer::Skip);
        if self.tracker.step(true, true, now_ms).verdict == SkipVerdict::LetGameSkip {
            self.close();
            return (
                PopVerdict::Run,
                VoteStep {
                    event: Some(VoteEvent::SkipAgreed),
                    ..VoteStep::default()
                },
            );
        }
        let _ = self.tracker.resend_due(now_ms);
        (
            PopVerdict::Drop,
            VoteStep {
                send: Some(SentAnswer {
                    prompt: prompt.prompt,
                    skip: true,
                }),
                event: Some(VoteEvent::AskedAndHeld),
                ..VoteStep::default()
            },
        )
    }

    fn guest_answered(
        &mut self,
        prompt: SkipPrompt,
        answer: PromptAnswer,
        now_ms: u64,
    ) -> VoteStep {
        self.page_opened_here = false;
        if answer == PromptAnswer::Other {
            self.close();
            return VoteStep {
                send: Some(SentAnswer {
                    prompt: prompt.prompt,
                    skip: false,
                }),
                event: Some(VoteEvent::DeclinedHere),
                ..VoteStep::default()
            };
        }
        self.ask_here(prompt, now_ms, VoteEvent::AskedAndHeld)
    }

    fn host_tick(&mut self, view: VoteView) -> VoteStep {
        let waited_ms = view.now_ms.saturating_sub(self.opened_ms);
        if self.local_answer == Some(PromptAnswer::Skip) {
            if self.tracker.step(false, true, view.now_ms).verdict == SkipVerdict::SkipNow {
                return self.fire(PromptAnswer::Skip, VoteEvent::SkipAgreed, false);
            }
            if waited_ms >= SKIP_PROMPT_VOTE_GIVES_UP_AFTER_MS {
                return self.fire(PromptAnswer::Other, VoteEvent::GaveUp, false);
            }
            return VoteStep::default();
        }
        let partner_voted = self.partner_declined || self.tracker.partner_ask_alive(view.now_ms);
        if partner_voted && view.page_shown_here && view.unattended {
            let answer = if self.partner_declined {
                PromptAnswer::Other
            } else {
                PromptAnswer::Skip
            };
            return self.fire(answer, VoteEvent::AnsweredForAbsentPlayer, true);
        }
        if waited_ms >= SKIP_PROMPT_VOTE_GIVES_UP_AFTER_MS && !view.page_shown_here {
            self.close();
        }
        VoteStep::default()
    }

    fn guest_tick(&mut self, prompt: SkipPrompt, view: VoteView) -> VoteStep {
        let waited_ms = view.now_ms.saturating_sub(self.opened_ms);
        if waited_ms >= SKIP_PROMPT_VOTE_GIVES_UP_AFTER_MS + PARTNER_ASK_LIFETIME_MS {
            return VoteStep {
                close_page: self.close(),
                event: Some(VoteEvent::GaveUp),
                ..VoteStep::default()
            };
        }
        if self.local_answer.is_some() || !self.tracker.partner_ask_alive(view.now_ms) {
            return VoteStep::default();
        }
        if view.unattended {
            // Nobody is at this window: its absent player agrees and closes whatever skip page is up.
            self.page_opened_here = false;
            let mut step = self.ask_here(prompt, view.now_ms, VoteEvent::AnsweredForAbsentPlayer);
            step.close_page = view.page_shown_here;
            return step;
        }
        if !view.page_shown_here && !self.page_opened_here && view.dialog_free {
            self.page_opened_here = true;
            return VoteStep {
                open_page: Some(prompt.prompt),
                ..VoteStep::default()
            };
        }
        VoteStep::default()
    }

    fn ask_here(&mut self, prompt: SkipPrompt, now_ms: u64, event: VoteEvent) -> VoteStep {
        self.local_answer = Some(PromptAnswer::Skip);
        let _ = self.tracker.step(true, true, now_ms);
        let _ = self.tracker.resend_due(now_ms);
        VoteStep {
            send: Some(SentAnswer {
                prompt: prompt.prompt,
                skip: true,
            }),
            event: Some(event),
            ..VoteStep::default()
        }
    }

    fn fire(&mut self, answer: PromptAnswer, event: VoteEvent, close_own_page: bool) -> VoteStep {
        let Some(prompt) = self.prompt else {
            return VoteStep::default();
        };
        let callback = prompt.callback_of(answer);
        let close_page = self.close() || close_own_page;
        self.release = Some(callback);
        VoteStep {
            fire: Some(callback),
            close_page,
            event: Some(event),
            ..VoteStep::default()
        }
    }

    fn open(&mut self, prompt: SkipPrompt, now_ms: u64) {
        if self.is_open_for(prompt.prompt) {
            return;
        }
        *self = Self {
            prompt: Some(prompt),
            opened_ms: now_ms,
            release: self.release,
            ..Self::new()
        };
        self.tracker.observe_scene(true, false);
    }

    fn close(&mut self) -> bool {
        let close_page = self.page_opened_here;
        *self = Self {
            release: self.release,
            ..Self::new()
        };
        close_page
    }

    fn is_open_for(&self, prompt: u16) -> bool {
        self.prompt.is_some_and(|open| open.prompt == prompt)
    }
}
