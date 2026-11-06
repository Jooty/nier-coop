use rc_proto::DialogueLine;

use crate::dialogue::{MIRROR_OPEN_RETRY_MS, TALK_JOIN_RANGE_UNITS, TALK_LEAVE_EXTRA_UNITS};

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
}

/// One tick's view of the partner's talk and of this game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MirrorInput {
    pub partner_line: Option<DialogueLine>,
    /// Same zone as the talker, in play, no menu, cutscene or loading.
    pub can_show: bool,
    pub window_open: bool,
    pub own_talk: Option<DialogueLine>,
    pub now_ms: u64,
}

/// Follows the partner's talk in this game's own talk window, line by line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartnerTalkMirror {
    shown: Option<DialogueLine>,
    retry_open_at_ms: u64,
    joined_own_talk: Option<u32>,
}

impl PartnerTalkMirror {
    pub const fn new() -> Self {
        Self {
            shown: None,
            retry_open_at_ms: 0,
            joined_own_talk: None,
        }
    }

    pub fn is_showing(&self) -> bool {
        self.shown.is_some()
    }

    pub fn step(&mut self, input: MirrorInput) -> Option<MirrorAction> {
        if let Some(own) = input.own_talk {
            self.shown = None;
            return self.join_own_talk(own, input.partner_line);
        }
        self.joined_own_talk = None;
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
