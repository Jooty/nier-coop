use rc_proto::{DialogueLine, DialogueUpdate};

use crate::dialogue::{
    FOLLOW_HOST_TALK_WAIT_MS, MIRROR_OPEN_RETRY_MS, OWN_ENDED_TALK_NOT_MIRRORED_MS,
    TALK_JOIN_RANGE_UNITS, TALK_LEAVE_EXTRA_UNITS,
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
    /// The host never showed this talk: the guest's own talk goes on by the guest's buttons.
    FollowGaveUp(u32),
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

/// One tick's view of the partner's talk and of this game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MirrorInput {
    pub partner_line: Option<DialogueLine>,
    /// Same zone as the talker, in play, no menu, cutscene or loading.
    pub can_show: bool,
    pub window_open: bool,
    pub own_talk: Option<DialogueLine>,
    pub leader: TalkLeader,
    pub now_ms: u64,
}

/// The partner's latest talk line, kept from the moment it arrives until the partner's talk ends,
/// so a guest whose own window opens a tick later still sees that the host got there first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PartnerTalkMemory {
    line: Option<DialogueLine>,
}

impl PartnerTalkMemory {
    pub const fn new() -> Self {
        Self { line: None }
    }

    pub fn receive(&mut self, update: DialogueUpdate) {
        self.line = match update {
            DialogueUpdate::Line(line) => Some(line),
            DialogueUpdate::Ended => None,
        };
    }

    pub fn forget(&mut self) {
        self.line = None;
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
                TalkLeader::Partner => self.follow_host(own, input.partner_line, input.now_ms),
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
            });
        }
        let followed = self.following.as_mut()?;
        if followed.gave_up {
            return None;
        }
        let host_on_it = partner_line.filter(|partner| partner.talk_id == own.talk_id);
        if let Some(host) = host_on_it {
            followed.host_seen_on_it = true;
            if fresh {
                return Some(MirrorAction::FollowStarted(own.talk_id));
            }
            return (host.line != own.line).then_some(MirrorAction::FollowLine(host.line));
        }
        if followed.host_seen_on_it {
            followed.gave_up = true;
            return Some(MirrorAction::FinishOwnTalk(own.talk_id));
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
