use crate::event_follow::guest_side::GuestSide;
use crate::event_follow::host_side::HostSide;
use crate::event_follow::{
    EventAsk, EventAskWhat, EventCall, EventId, EventReadings, EventSend, EventStep,
    MOST_SCRIPT_PLAY_REQUESTS_REMEMBERED, PARTNER_REPORT_STALE_FOR_A_SKIP_VOTE_MS, PartnerReport,
    PlayVerdict, PlayedHere, PlayingHere, RunsForThePartner,
    SCRIPT_PLAY_REQUEST_COUNTS_FOR_ITS_START_MS, SeenEvents, SkipVerdict, SkipVote, StartOrigin,
    VoteView, ended_here_line, replay_skipped_line, started_here_line,
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
    played: PlayedHere,
    runs: RunsForThePartner,
    host: HostSide,
    guest: GuestSide,
    before_the_next_step: EventStep,
    skip_vote_on: bool,
    vote: SkipVote,
    partner_report: Option<(PartnerReport, u64)>,
}

impl EventMachine {
    pub fn new(starts_by_data_id: bool) -> Self {
        Self {
            starts_by_data_id,
            ..Self::default()
        }
    }

    /// With the vote on, a skip runs once both players asked (replaces the either-player skip).
    pub fn with_skip_vote(self, skip_vote_on: bool) -> Self {
        Self {
            skip_vote_on,
            ..self
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
        let told = origin.is_mirrored() && self.linked_role == JumpRole::Host;
        self.playing.retain(|here| here.id != id);
        self.playing.push(PlayingHere {
            id,
            since_ms: now_ms,
            origin,
            told,
        });
        step.logs.push(started_here_line(id, origin));
        if origin.is_mirrored() {
            self.vote.event_started(id);
        }
        if origin == StartOrigin::Replay {
            step.calls.push(EventCall::Skip);
            step.logs.push(replay_skipped_line(id));
        }
        self.played.note(id, now_ms);
        if told {
            step.sends.push(EventSend::Started(id));
        }
        if origin.is_mirrored() && self.linked_role == JumpRole::Guest {
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
            step.sends.push(EventSend::Ended { id, skipped });
        }
        self.runs.end(id, step);
        self.guest.own_end(id);
        self.vote.event_ended(id);
    }

    /// The player chose skip in the event pause (the call is not ours).
    pub fn on_player_skip(&mut self, now_ms: u64) -> SkipVerdict {
        if self.skip_vote_on && self.rule_role != JumpRole::Solo {
            let view = self.vote_view(now_ms);
            return self
                .vote
                .own_skip(&view, now_ms, &mut self.before_the_next_step);
        }
        if self.skip_vote_on || self.rule_role != JumpRole::Guest {
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
        self.played.follow_place(readings.current_place, now_ms);
        self.script_asked.retain(|(_, at_ms)| {
            now_ms.saturating_sub(*at_ms) <= SCRIPT_PLAY_REQUEST_COUNTS_FOR_ITS_START_MS
        });
        self.seen.forget_old(now_ms);
        match linked_role {
            JumpRole::Host => self.host.answer(
                readings,
                &self.playing,
                &self.played,
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
                &self.played,
                self.starts_by_data_id,
                &mut step,
            );
        }
        self.runs.advance(readings, &mut self.seen, &mut step);
        if linked_role == JumpRole::Solo {
            step.sends.clear();
        }
        if rule_role != JumpRole::Guest {
            // Only the guest's own asks go: the host's "already played" answer stays.
            step.sends.retain(|send| {
                !matches!(
                    send,
                    EventSend::Ask(EventAsk {
                        what: EventAskWhat::Start | EventAskWhat::Skip,
                        ..
                    })
                )
            });
        }
        self.partner_report = readings.partner_report.map(|report| (report, now_ms));
        if self.skip_vote_on {
            if linked_role != JumpRole::Solo {
                for id in &readings.skip_asks_heard {
                    self.vote.heard(*id, now_ms, &mut step);
                }
            }
            let view = self.vote_view(now_ms);
            step.skip_prompt = self
                .vote
                .step(&view, readings.local_unattended, now_ms, &mut step);
        }
        step.playing_mirrored_event = self
            .playing
            .iter()
            .rev()
            .find(|here| here.origin.is_mirrored())
            .map(|here| here.id);
        step
    }

    fn vote_view(&self, now_ms: u64) -> VoteView {
        let event = self
            .playing
            .iter()
            .rev()
            .find(|here| here.origin.is_mirrored())
            .map(|here| here.id);
        let partner = match (self.rule_role, self.partner_report, event) {
            (JumpRole::Solo, _, _) => Err("the link is down or a copy is not in play".to_owned()),
            (_, None, _) => Err("no state report from the partner".to_owned()),
            (_, Some((report, read_at_ms)), Some(id)) => {
                let age_ms = report.age_ms + now_ms.saturating_sub(read_at_ms);
                if age_ms > PARTNER_REPORT_STALE_FOR_A_SKIP_VOTE_MS {
                    Err(format!("its state report is {age_ms} ms old"))
                } else if report.playing != Some(id) {
                    Err("it does not play this event".to_owned())
                } else {
                    Ok(report.unattended)
                }
            }
            (_, Some(_), None) => Err("this copy plays no story event".to_owned()),
        };
        VoteView {
            role: self.rule_role,
            event,
            partner,
        }
    }
}
