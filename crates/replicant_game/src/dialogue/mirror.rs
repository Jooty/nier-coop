use replicant_proto::{DialogueLine, DialogueUpdate};

use crate::dialogue::{
    CHOICE_FOLLOW_GIVES_THE_GUEST_ITS_BUTTONS_AFTER_MS, FOLLOW_HOST_TALK_WAIT_MS,
    MIRROR_OPEN_RETRY_MS, OWN_ENDED_TALK_NOT_MIRRORED_MS, TALK_JOIN_RANGE_UNITS,
    TALK_LEAVE_EXTRA_UNITS,
};

/// What the partner's talk window must do this tick to follow the talker's line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MirrorAction {
    /// Open the window on `talk_id` and go straight to `line` (a late joiner starts mid-talk).
    Open(DialogueLine),
    /// Move the open mirrored window to this line.
    ShowLine(u16),
    Close,
    /// This player started the same talk on their own: move their window to the talker's line.
    JumpOwnTalk(u16),
    /// The guest's own copy of a talk the host leads waits on the host: buttons do nothing.
    FollowStarted(u32),
    /// Move the guest's own copy of the host's talk to the host's line.
    FollowLine(u16),
    /// The host finished or left this talk: close the guest's copy so its script moves on.
    FinishOwnTalk(u32),
    /// The host stepped from this talk straight into another one: the guest's copy takes the same
    /// step, so its game opens the following talk itself.
    ChainOwnTalk(DialogueLine),
    /// The host never showed this talk: the guest's own talk goes on by the guest's buttons.
    FollowGaveUp(u32),
    /// The host left the talk while the guest's copy was at an answer prompt that never got the
    /// host's answer: the guest's copy goes on by the guest's buttons.
    ChoiceGaveUp(u32),
    /// The partner is still on a talk this player just finished: not opened again as a mirror.
    SkipEndedTalk(u32),
}

/// Who moves a talk both players have open on their own games. Only a partner within talk range
/// leads or follows; otherwise each player reads at their own pace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TalkLeader {
    Local,
    Partner,
    Either,
}

/// A talk the local player started with the talk command is theirs alone: it is never led by
/// the partner. A copy a script opened (stream entry, relayed call, shared event) follows `base`.
pub fn leader_for_own_talk(base: TalkLeader, started_by_player_command: bool) -> TalkLeader {
    if started_by_player_command && base == TalkLeader::Partner {
        TalkLeader::Local
    } else {
        base
    }
}

/// One tick's view of the partner's talk and of this game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MirrorInput {
    pub partner_line: Option<DialogueLine>,
    pub partner_ended_talk: Option<u32>,
    /// Same zone as the talker, in play, no menu, cutscene or loading.
    pub can_show: bool,
    pub window_open: bool,
    pub own_talk: Option<DialogueLine>,
    pub leader: TalkLeader,
    /// The own copy is at an answer prompt, or the host's answer to it has not been taken yet.
    pub answer_open: bool,
    pub now_ms: u64,
}

/// The partner's latest talk line, kept from the moment it arrives until the partner's talk ends,
/// so a guest whose own window opens a tick later still sees that the host got there first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PartnerTalkMemory {
    line: Option<DialogueLine>,
    last_ended: Option<(u32, u64)>,
}

impl PartnerTalkMemory {
    pub const fn new() -> Self {
        Self {
            line: None,
            last_ended: None,
        }
    }

    pub fn receive(&mut self, update: DialogueUpdate, now_ms: u64) {
        match update {
            DialogueUpdate::Line(line) => {
                self.line = Some(line);
                self.last_ended = None;
            }
            DialogueUpdate::Ended => {
                if let Some(line) = self.line.take() {
                    self.last_ended = Some((line.talk_id, now_ms));
                }
            }
        }
    }

    pub fn forget(&mut self) {
        self.line = None;
        self.last_ended = None;
    }

    pub fn current_line(&self) -> Option<DialogueLine> {
        self.line
    }

    /// The talk the host left within the wait before this tick, when the guest's own copy is that
    /// same talk: the host already left, so the guest's copy closes at once.
    pub fn ended_for_follow(
        &self,
        own_talk: Option<DialogueLine>,
        leader: TalkLeader,
        now_ms: u64,
    ) -> Option<u32> {
        let own = own_talk.filter(|_| leader == TalkLeader::Partner)?;
        self.last_ended
            .filter(|&(talk_id, ended_ms)| {
                talk_id == own.talk_id
                    && now_ms.saturating_sub(ended_ms) <= FOLLOW_HOST_TALK_WAIT_MS
            })
            .map(|(talk_id, _)| talk_id)
    }

    /// The line the mirror may see. Only a guest with its own copy of a talk the host leads
    /// follows; a partner talk this player does not have open stays unshown.
    pub fn for_follow(
        &self,
        own_talk: Option<DialogueLine>,
        leader: TalkLeader,
    ) -> Option<DialogueLine> {
        self.line
            .filter(|_| own_talk.is_some() && leader == TalkLeader::Partner)
    }
}

/// Follows the partner's talk in this game's own talk window, line by line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartnerTalkMirror {
    shown: Option<DialogueLine>,
    retry_open_at_ms: u64,
    joined_own_talk: Option<u32>,
    following: Option<FollowedTalk>,
    last_own_talk: Option<u32>,
    own_ended: Option<EndedTalk>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EndedTalk {
    talk_id: u32,
    at_ms: u64,
    skip_told: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FollowedTalk {
    talk_id: u32,
    since_ms: u64,
    host_seen_on_it: bool,
    gave_up: bool,
    finish_deferred_since_ms: Option<u64>,
    answer_seen: bool,
}

impl PartnerTalkMirror {
    pub const fn new() -> Self {
        Self {
            shown: None,
            retry_open_at_ms: 0,
            joined_own_talk: None,
            following: None,
            last_own_talk: None,
            own_ended: None,
        }
    }

    pub fn is_showing(&self) -> bool {
        self.shown.is_some()
    }

    /// The guest's own talk window waits on the host's lines, so its buttons must not move it.
    pub fn holds_own_talk(&self) -> bool {
        self.following.is_some_and(|followed| !followed.gave_up)
    }

    pub fn step(&mut self, input: MirrorInput) -> Option<MirrorAction> {
        if let Some(own) = input.own_talk {
            self.shown = None;
            self.last_own_talk = Some(own.talk_id);
            self.own_ended = None;
            return match input.leader {
                TalkLeader::Partner => self.follow_host(
                    own,
                    input.partner_line,
                    input.partner_ended_talk,
                    input.answer_open,
                    input.now_ms,
                ),
                TalkLeader::Local => {
                    self.following = None;
                    None
                }
                TalkLeader::Either => {
                    self.following = None;
                    self.join_own_talk(own, input.partner_line)
                }
            };
        }
        self.joined_own_talk = None;
        self.following = None;
        if let Some(talk_id) = self.last_own_talk.take() {
            self.own_ended = Some(EndedTalk {
                talk_id,
                at_ms: input.now_ms,
                skip_told: false,
            });
        }
        if let Some(skip) = self.skip_ended_talk(input.partner_line, input.now_ms) {
            return skip;
        }
        if self.shown.is_some() && !input.window_open {
            self.shown = None;
        }
        let wanted = input.partner_line.filter(|_| input.can_show);
        match (self.shown, wanted) {
            (None, None) => None,
            (None, Some(line)) => {
                if input.window_open || input.now_ms < self.retry_open_at_ms {
                    return None;
                }
                self.shown = Some(line);
                self.retry_open_at_ms = input.now_ms + MIRROR_OPEN_RETRY_MS;
                Some(MirrorAction::Open(line))
            }
            (Some(shown), Some(line)) if shown.talk_id == line.talk_id => {
                if shown.line == line.line {
                    return None;
                }
                self.shown = Some(line);
                Some(MirrorAction::ShowLine(line.line))
            }
            (Some(_), _) => {
                self.shown = None;
                Some(MirrorAction::Close)
            }
        }
    }

    fn skip_ended_talk(
        &mut self,
        partner_line: Option<DialogueLine>,
        now_ms: u64,
    ) -> Option<Option<MirrorAction>> {
        let ended = self.own_ended.as_mut()?;
        let partner_still_on_it = partner_line.is_some_and(|line| line.talk_id == ended.talk_id);
        let fresh = now_ms.saturating_sub(ended.at_ms) < OWN_ENDED_TALK_NOT_MIRRORED_MS;
        if !partner_still_on_it || !fresh || self.shown.is_some() {
            self.own_ended = None;
            return None;
        }
        if ended.skip_told {
            return Some(None);
        }
        ended.skip_told = true;
        Some(Some(MirrorAction::SkipEndedTalk(ended.talk_id)))
    }

    fn follow_host(
        &mut self,
        own: DialogueLine,
        partner_line: Option<DialogueLine>,
        partner_ended_talk: Option<u32>,
        answer_open: bool,
        now_ms: u64,
    ) -> Option<MirrorAction> {
        let fresh = self
            .following
            .is_none_or(|followed| followed.talk_id != own.talk_id);
        if fresh {
            self.following = Some(FollowedTalk {
                talk_id: own.talk_id,
                since_ms: now_ms,
                host_seen_on_it: false,
                gave_up: false,
                finish_deferred_since_ms: None,
                answer_seen: false,
            });
        }
        let followed = self.following.as_mut()?;
        followed.answer_seen |= answer_open;
        if followed.gave_up {
            return None;
        }
        let host_left_before_open =
            fresh && partner_line.is_none() && partner_ended_talk == Some(own.talk_id);
        if host_left_before_open {
            return Self::finish_own_talk(followed, own.talk_id, None, answer_open, now_ms);
        }
        let host_on_it = partner_line.filter(|partner| partner.talk_id == own.talk_id);
        if let Some(host) = host_on_it {
            followed.host_seen_on_it = true;
            followed.finish_deferred_since_ms = None;
            if fresh {
                return Some(MirrorAction::FollowStarted(own.talk_id));
            }
            return (host.line != own.line).then_some(MirrorAction::FollowLine(host.line));
        }
        if followed.host_seen_on_it {
            let chained_to = partner_line.filter(|partner| partner.talk_id != own.talk_id);
            return Self::finish_own_talk(followed, own.talk_id, chained_to, answer_open, now_ms);
        }
        if fresh {
            return Some(MirrorAction::FollowStarted(own.talk_id));
        }
        if now_ms.saturating_sub(followed.since_ms) >= FOLLOW_HOST_TALK_WAIT_MS {
            followed.gave_up = true;
            return Some(MirrorAction::FollowGaveUp(own.talk_id));
        }
        None
    }

    fn finish_own_talk(
        followed: &mut FollowedTalk,
        talk_id: u32,
        chained_to: Option<DialogueLine>,
        answer_open: bool,
        now_ms: u64,
    ) -> Option<MirrorAction> {
        if !answer_open {
            followed.gave_up = true;
            return Some(match chained_to.filter(|_| !followed.answer_seen) {
                Some(next) => MirrorAction::ChainOwnTalk(next),
                None => MirrorAction::FinishOwnTalk(talk_id),
            });
        }
        let since_ms = *followed.finish_deferred_since_ms.get_or_insert(now_ms);
        if now_ms.saturating_sub(since_ms) >= CHOICE_FOLLOW_GIVES_THE_GUEST_ITS_BUTTONS_AFTER_MS {
            followed.gave_up = true;
            return Some(MirrorAction::ChoiceGaveUp(talk_id));
        }
        None
    }

    /// The guest's copy goes on by its own buttons from now on.
    pub fn release_followed_talk(&mut self) {
        if let Some(followed) = self.following.as_mut() {
            followed.gave_up = true;
        }
    }

    fn join_own_talk(
        &mut self,
        own: DialogueLine,
        partner_line: Option<DialogueLine>,
    ) -> Option<MirrorAction> {
        if self.joined_own_talk == Some(own.talk_id) {
            return None;
        }
        self.joined_own_talk = Some(own.talk_id);
        let partner = partner_line.filter(|partner| partner.talk_id == own.talk_id)?;
        (partner.line > own.line).then_some(MirrorAction::JumpOwnTalk(partner.line))
    }
}

/// Whether this player is close enough to the talker to join their talk. A window already
/// showing stays a little longer, so standing on the edge does not open and close it.
pub fn within_talk_range(distance_units: f32, already_showing: bool) -> bool {
    let range = if already_showing {
        TALK_JOIN_RANGE_UNITS + TALK_LEAVE_EXTRA_UNITS
    } else {
        TALK_JOIN_RANGE_UNITS
    };
    distance_units.is_finite() && distance_units <= range
}

/// Whether a talk line's tag callbacks may run on this machine. Only the talker runs them.
pub fn talk_tags_run_here(mirroring: bool, mirror_action_running: bool) -> bool {
    !mirroring && !mirror_action_running
}
