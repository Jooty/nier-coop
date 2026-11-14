use crate::event_follow::{
    EventCall, EventId, EventSend, EventStep, OWN_SKIP_ASK_LASTS_MS, PARTNER_SKIP_ASK_LIFETIME_MS,
    SKIP_ASK_RESEND_MS, SkipPrompt, SkipVerdict, Unattended, agreed_as_unattended_line,
    agreed_by_own_input_line, attendance_line, own_skip_ask_lapsed_line, own_skip_ask_line,
    partner_counted_as_agreeing_line, partner_does_not_count_line, partner_skip_ask_heard_line,
    partner_skip_ask_lapsed_line, vote_passed_line,
};
use crate::travel::JumpRole;

/// What a skip vote needs to know of this copy and the partner at one moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoteView {
    pub role: JumpRole,
    pub event: Option<EventId>,
    /// `Ok(partner reports unattended)` while the partner counts, else why it does not.
    pub partner: Result<bool, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OwnAsk {
    id: EventId,
    since_ms: u64,
    sent_at_ms: Option<u64>,
}

/// A cutscene skip runs once both players asked; an unattended copy counts as asking.
#[derive(Debug, Clone, Default)]
pub struct SkipVote {
    own_ask: Option<OwnAsk>,
    partner_ask: Option<(EventId, u64)>,
    passed_for: Option<EventId>,
    attendance: Option<Unattended>,
}

impl SkipVote {
    pub fn partner_ask_alive(&self, id: EventId, now_ms: u64) -> bool {
        self.partner_ask.is_some_and(|(asked, heard_at_ms)| {
            asked == id && now_ms.saturating_sub(heard_at_ms) < PARTNER_SKIP_ASK_LIFETIME_MS
        })
    }

    pub fn own_skip(&mut self, view: &VoteView, now_ms: u64, step: &mut EventStep) -> SkipVerdict {
        let Some(id) = view.event else {
            return SkipVerdict::Pass;
        };
        let runs_here = view.role == JumpRole::Host;
        if self.passed_for == Some(id) {
            return if runs_here {
                SkipVerdict::Pass
            } else {
                SkipVerdict::Swallow
            };
        }
        let partner_unattended = match &view.partner {
            Err(why) => {
                step.logs.push(partner_does_not_count_line(id, why));
                self.own_ask = None;
                self.passed_for = Some(id);
                return SkipVerdict::Pass;
            }
            Ok(partner_unattended) => *partner_unattended,
        };
        let partner_asked = self.partner_ask_alive(id, now_ms);
        self.own_ask = Some(OwnAsk {
            id,
            since_ms: now_ms,
            sent_at_ms: None,
        });
        if !partner_asked && !partner_unattended {
            step.logs.push(own_skip_ask_line(id));
            self.send_due_ask(view, now_ms, step);
            return SkipVerdict::Swallow;
        }
        if partner_asked {
            step.logs.push(agreed_by_own_input_line(id));
        } else {
            step.logs.push(partner_counted_as_agreeing_line(id));
        }
        self.passed_for = Some(id);
        step.logs.push(vote_passed_line(id, runs_here));
        if runs_here {
            SkipVerdict::Pass
        } else {
            self.send_due_ask(view, now_ms, step);
            SkipVerdict::Swallow
        }
    }

    pub fn heard(&mut self, id: EventId, now_ms: u64, step: &mut EventStep) {
        if !self.partner_ask_alive(id, now_ms) {
            step.logs.push(partner_skip_ask_heard_line(id));
        }
        self.partner_ask = Some((id, now_ms));
    }

    pub fn step(
        &mut self,
        view: &VoteView,
        local: Option<Unattended>,
        now_ms: u64,
        step: &mut EventStep,
    ) -> Option<SkipPrompt> {
        if local != self.attendance {
            self.attendance = local;
            step.logs.push(attendance_line(local));
        }
        if let Some((id, heard_at_ms)) = self.partner_ask
            && now_ms.saturating_sub(heard_at_ms) >= PARTNER_SKIP_ASK_LIFETIME_MS
        {
            self.partner_ask = None;
            if self.passed_for != Some(id) {
                step.logs.push(partner_skip_ask_lapsed_line(id));
            }
        }
        if let Some(ask) = self.own_ask {
            let waited_ms = now_ms.saturating_sub(ask.since_ms);
            if view.event != Some(ask.id) {
                self.own_ask = None;
            } else if self.passed_for != Some(ask.id) && waited_ms >= OWN_SKIP_ASK_LASTS_MS {
                self.own_ask = None;
                step.logs.push(own_skip_ask_lapsed_line(ask.id, waited_ms));
            }
        }
        let id = view.event?;
        if self.passed_for != Some(id) {
            self.decide(id, view, local, now_ms, step);
        }
        self.send_due_ask(view, now_ms, step);
        if self.passed_for == Some(id) {
            None
        } else if self.own_ask.is_some() {
            Some(SkipPrompt::WaitingForThePartner)
        } else if self.partner_ask_alive(id, now_ms) {
            Some(SkipPrompt::PartnerWantsToSkip)
        } else {
            None
        }
    }

    pub fn event_started(&mut self, id: EventId) {
        self.own_ask = None;
        self.passed_for = None;
        if self.partner_ask.is_some_and(|(asked, _)| asked != id) {
            self.partner_ask = None;
        }
    }

    pub fn event_ended(&mut self, id: EventId) {
        if self.own_ask.is_some_and(|ask| ask.id == id) {
            self.own_ask = None;
        }
        if self.partner_ask.is_some_and(|(asked, _)| asked == id) {
            self.partner_ask = None;
        }
        if self.passed_for == Some(id) {
            self.passed_for = None;
        }
    }

    fn decide(
        &mut self,
        id: EventId,
        view: &VoteView,
        local: Option<Unattended>,
        now_ms: u64,
        step: &mut EventStep,
    ) {
        let partner_asked = self.partner_ask_alive(id, now_ms);
        if self.own_ask.is_none()
            && partner_asked
            && view.partner.is_ok()
            && let Some(why) = local
        {
            self.own_ask = Some(OwnAsk {
                id,
                since_ms: now_ms,
                sent_at_ms: None,
            });
            step.logs.push(agreed_as_unattended_line(id, why));
        }
        if self.own_ask.is_none() {
            return;
        }
        let runs_here = view.role == JumpRole::Host;
        match &view.partner {
            Err(why) => {
                step.logs.push(partner_does_not_count_line(id, why));
                self.own_ask = None;
                self.passed_for = Some(id);
                step.calls.push(EventCall::Skip);
            }
            Ok(partner_unattended) if partner_asked || *partner_unattended => {
                if !partner_asked {
                    step.logs.push(partner_counted_as_agreeing_line(id));
                }
                self.passed_for = Some(id);
                step.logs.push(vote_passed_line(id, runs_here));
                if runs_here {
                    step.calls.push(EventCall::Skip);
                }
            }
            Ok(_) => {}
        }
    }

    fn send_due_ask(&mut self, view: &VoteView, now_ms: u64, step: &mut EventStep) {
        let Some(ask) = self.own_ask.as_mut() else {
            return;
        };
        // After a passed vote only the guest goes on asking: the host must hear it to run the skip.
        let still_asking = self.passed_for != Some(ask.id) || view.role == JumpRole::Guest;
        let due = ask
            .sent_at_ms
            .is_none_or(|sent_at_ms| now_ms.saturating_sub(sent_at_ms) >= SKIP_ASK_RESEND_MS);
        if still_asking && due && view.event == Some(ask.id) && view.partner.is_ok() {
            ask.sent_at_ms = Some(now_ms);
            step.sends.push(EventSend::SkipAsk(ask.id));
        }
    }
}
