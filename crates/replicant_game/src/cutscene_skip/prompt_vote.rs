use crate::cutscene_skip::{
    CutsceneSkipTracker, PARTNER_ASK_LIFETIME_MS, PromptAnswer, SKIP_PROMPT_VOTE_GIVES_UP_AFTER_MS,
    SkipPrompt, SkipVerdict, skip_prompt, skip_prompt_answer, skip_prompt_ended_by,
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
    SkipAgreed,
    SkipAlone,
    DeclinedHere,
    DeclinedByPartner,
    GaveUp,
    HostAnswerRan,
    HostChoseHere,
    SequenceEnded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoteView {
    pub host: bool,
    /// The partner is connected, in the same place and in the prompt's sequence: only then does
    /// it have a say, otherwise this player's answer goes through at once.
    pub partner_present: bool,
    pub page_shown_here: bool,
    pub dialog_free: bool,
    pub now_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SentAnswer {
    pub prompt: u16,
    pub skip: bool,
}

/// What one vote step asks of the game: `send` is this player's answer to the partner, `verdict`
/// the host's end of the vote for the guest, `fire` the answer callback to queue here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VoteStep {
    pub send: Option<SentAnswer>,
    pub verdict: Option<SentAnswer>,
    pub fire: Option<&'static str>,
    pub open_page: Option<u16>,
    pub close_page: bool,
    pub event: Option<VoteEvent>,
}

/// The scene skip vote (`CutsceneSkipTracker`) run for a skip of a sequence both players are in.
/// Nobody answers for a player whose window is unattended: each player answers its own prompt
/// or presses its own skip. The host decides and sends its verdict; the guest runs the same answer.
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
        if let Some(ended) = skip_prompt_ended_by(callback) {
            if !self.is_open_for(ended.prompt) {
                return (PopVerdict::Run, VoteStep::default());
            }
            let close_page = self.close();
            return (
                PopVerdict::Run,
                VoteStep {
                    close_page,
                    event: Some(VoteEvent::SequenceEnded),
                    ..VoteStep::default()
                },
            );
        }
        let Some((prompt, answer)) = skip_prompt_answer(callback) else {
            return (PopVerdict::Run, VoteStep::default());
        };
        if self.release.is_some() && self.release == prompt.callback_of(answer) {
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
            self.guest_answered(prompt, answer, view.now_ms)
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
                return self.fire(prompt, PromptAnswer::Other, VoteEvent::DeclinedByPartner);
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

    /// The guest's side of the host's verdict: a skip runs here in any case, so the sequence
    /// ends on both; the other answer runs only where this game holds its own skip.
    pub fn host_verdict(&mut self, prompt: u16, skip: bool) -> VoteStep {
        let Some(prompt) = skip_prompt(prompt) else {
            return VoteStep::default();
        };
        let held_here =
            self.is_open_for(prompt.prompt) && self.local_answer == Some(PromptAnswer::Skip);
        if skip || held_here {
            let answer = if skip {
                PromptAnswer::Skip
            } else {
                PromptAnswer::Other
            };
            return self.fire(prompt, answer, VoteEvent::HostChoseHere);
        }
        if !self.is_open_for(prompt.prompt) {
            return VoteStep::default();
        }
        VoteStep {
            close_page: self.close(),
            event: Some(VoteEvent::HostChoseHere),
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
            if self.local_answer == Some(PromptAnswer::Skip) {
                let mut step = self.fire(prompt, PromptAnswer::Skip, VoteEvent::SkipAlone);
                // A partner out of the sequence gets no verdict: the skip is this game's alone.
                step.verdict = None;
                return step;
            }
            return VoteStep {
                close_page: self.close(),
                ..VoteStep::default()
            };
        }
        let mut step = if view.host {
            self.host_tick(prompt, view)
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
                    verdict: Some(SentAnswer {
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
                self.fire(prompt, PromptAnswer::Other, VoteEvent::DeclinedByPartner),
            );
        }
        self.local_answer = Some(PromptAnswer::Skip);
        if self.tracker.step(true, true, now_ms).verdict == SkipVerdict::LetGameSkip {
            self.close();
            return (
                PopVerdict::Run,
                VoteStep {
                    verdict: Some(SentAnswer {
                        prompt: prompt.prompt,
                        skip: true,
                    }),
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
    ) -> (PopVerdict, VoteStep) {
        self.page_opened_here = false;
        if answer == PromptAnswer::Other {
            // A no ends the vote: it runs here at once, and the host turns its own skip into it.
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
        (PopVerdict::Drop, self.ask_here(prompt, now_ms))
    }

    fn host_tick(&mut self, prompt: SkipPrompt, view: VoteView) -> VoteStep {
        let waited_ms = view.now_ms.saturating_sub(self.opened_ms);
        if self.local_answer == Some(PromptAnswer::Skip) {
            if self.tracker.step(false, true, view.now_ms).verdict == SkipVerdict::SkipNow {
                return self.fire(prompt, PromptAnswer::Skip, VoteEvent::SkipAgreed);
            }
            if waited_ms >= SKIP_PROMPT_VOTE_GIVES_UP_AFTER_MS {
                return self.fire(prompt, PromptAnswer::Other, VoteEvent::GaveUp);
            }
            return VoteStep::default();
        }
        if waited_ms >= SKIP_PROMPT_VOTE_GIVES_UP_AFTER_MS && !view.page_shown_here {
            self.close();
        }
        VoteStep::default()
    }

    fn guest_tick(&mut self, prompt: SkipPrompt, view: VoteView) -> VoteStep {
        let waited_ms = view.now_ms.saturating_sub(self.opened_ms);
        if waited_ms >= SKIP_PROMPT_VOTE_GIVES_UP_AFTER_MS + PARTNER_ASK_LIFETIME_MS {
            if self.local_answer == Some(PromptAnswer::Skip) {
                // No verdict came: this game's held skip turns into the other answer, as the host's would.
                let mut step = self.fire(prompt, PromptAnswer::Other, VoteEvent::GaveUp);
                step.send = Some(SentAnswer {
                    prompt: prompt.prompt,
                    skip: false,
                });
                return step;
            }
            return VoteStep {
                close_page: self.close(),
                event: Some(VoteEvent::GaveUp),
                ..VoteStep::default()
            };
        }
        if self.local_answer.is_some() || !self.tracker.partner_ask_alive(view.now_ms) {
            return VoteStep::default();
        }
        if prompt.shows_page && !view.page_shown_here && !self.page_opened_here && view.dialog_free
        {
            self.page_opened_here = true;
            return VoteStep {
                open_page: Some(prompt.prompt),
                ..VoteStep::default()
            };
        }
        VoteStep::default()
    }

    fn ask_here(&mut self, prompt: SkipPrompt, now_ms: u64) -> VoteStep {
        self.local_answer = Some(PromptAnswer::Skip);
        let _ = self.tracker.step(true, true, now_ms);
        let _ = self.tracker.resend_due(now_ms);
        VoteStep {
            send: Some(SentAnswer {
                prompt: prompt.prompt,
                skip: true,
            }),
            event: Some(VoteEvent::AskedAndHeld),
            ..VoteStep::default()
        }
    }

    fn fire(&mut self, prompt: SkipPrompt, answer: PromptAnswer, event: VoteEvent) -> VoteStep {
        let callback = prompt.callback_of(answer);
        let close_page = self.close();
        if callback.is_some() {
            self.release = callback;
        }
        VoteStep {
            verdict: Some(SentAnswer {
                prompt: prompt.prompt,
                skip: answer == PromptAnswer::Skip,
            }),
            fire: callback,
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
