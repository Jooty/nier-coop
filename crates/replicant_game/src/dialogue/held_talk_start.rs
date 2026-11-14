use crate::dialogue::HELD_TALK_START_WAITS_FOR_THE_PARTNER_MS;

/// A talk start inside the partner's step that this game runs as a copy: the talk itself is the
/// partner's (shown here by the mirror), so the copy's script waits in the start until the
/// partner's talk is over, and its scene camera and placements come at the partner's pace.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeldTalkStart {
    held: Option<HeldTalk>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HeldTalk {
    native_block: u64,
    talk_id: u32,
    since_ms: u64,
    partner_was_on: bool,
    whole_talk: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeldTalkStep {
    /// The native keeps returning "busy".
    Wait,
    /// The native returns "done" and the copy's script goes on.
    Release(HeldTalkRelease),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeldTalkRelease {
    PartnerFinished,
    PartnerNeverShowed,
}

impl HeldTalkStart {
    pub const fn new() -> Self {
        Self { held: None }
    }

    pub fn holds(&self, native_block: u64) -> Option<u32> {
        self.held
            .filter(|held| held.native_block == native_block)
            .map(|held| held.talk_id)
    }

    pub fn begin(&mut self, native_block: u64, talk_id: u32, now_ms: u64) {
        self.held = Some(HeldTalk {
            native_block,
            talk_id,
            since_ms: now_ms,
            partner_was_on: false,
            whole_talk: false,
        });
    }

    /// As `begin`, for a copy of a step the host's guide leads: the host's talk chains further
    /// talks in the same window, and the copy waits until that window closes, not its first talk.
    pub fn begin_for_the_whole_talk(&mut self, native_block: u64, talk_id: u32, now_ms: u64) {
        self.begin(native_block, talk_id, now_ms);
        if let Some(held) = self.held.as_mut() {
            held.whole_talk = true;
        }
    }

    pub fn waits_for_the_whole_talk(&self) -> bool {
        self.held.is_some_and(|held| held.whole_talk)
    }

    /// One poll of the held native, with whether the partner's window shows that talk now and
    /// whether the partner ended it within the follow wait (this copy came late).
    pub fn poll(
        &mut self,
        partner_on_talk: bool,
        partner_just_ended_it: bool,
        now_ms: u64,
    ) -> HeldTalkStep {
        let Some(held) = self.held.as_mut() else {
            return HeldTalkStep::Release(HeldTalkRelease::PartnerFinished);
        };
        if partner_on_talk {
            held.partner_was_on = true;
            return HeldTalkStep::Wait;
        }
        let release = if held.partner_was_on || partner_just_ended_it {
            HeldTalkRelease::PartnerFinished
        } else if now_ms.saturating_sub(held.since_ms) >= HELD_TALK_START_WAITS_FOR_THE_PARTNER_MS {
            HeldTalkRelease::PartnerNeverShowed
        } else {
            return HeldTalkStep::Wait;
        };
        self.held = None;
        HeldTalkStep::Release(release)
    }

    pub fn forget(&mut self) {
        self.held = None;
    }
}

impl HeldTalkRelease {
    pub const fn describe(self) -> &'static str {
        match self {
            Self::PartnerFinished => "the partner's talk is over, your copy goes on",
            Self::PartnerNeverShowed => {
                "the partner never showed that talk, your copy goes on without it"
            }
        }
    }
}
