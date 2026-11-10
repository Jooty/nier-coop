use crate::boss_messages::{
    FIRST_MIRRORED_MESSAGE, LAST_MIRRORED_MESSAGE, MIXED_POSTER_MESSAGES_OUTSIDE_MIRRORED_RANGE,
    NEVER_MIRRORED_MESSAGES, PLAYER_CODE_POSTED_MESSAGES,
    STATE_STEP_ONLY_MESSAGES_OUTSIDE_MIRRORED_RANGE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MirrorVerdict {
    Mirror,
    NeverMirrored,
    MixedPoster,
    OutsideRange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    Disabled,
    NotGuest,
    NotInPlay,
    OtherPhase,
    NotMirrored,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuestVerdict {
    Raise,
    Skip(SkipReason),
}

#[derive(Clone, Copy, Debug)]
pub struct GuestFacts {
    pub enabled: bool,
    pub is_guest: bool,
    pub local_in_play: bool,
    pub local_phase: Option<u16>,
    pub message_phase: u16,
    pub message: u32,
}

pub fn mirror_verdict(message: u32) -> MirrorVerdict {
    if NEVER_MIRRORED_MESSAGES.contains(&message)
        || PLAYER_CODE_POSTED_MESSAGES.contains(&message)
    {
        MirrorVerdict::NeverMirrored
    } else if (FIRST_MIRRORED_MESSAGE..=LAST_MIRRORED_MESSAGE).contains(&message)
        || STATE_STEP_ONLY_MESSAGES_OUTSIDE_MIRRORED_RANGE.contains(&message)
    {
        MirrorVerdict::Mirror
    } else if MIXED_POSTER_MESSAGES_OUTSIDE_MIRRORED_RANGE.contains(&message) {
        MirrorVerdict::MixedPoster
    } else {
        MirrorVerdict::OutsideRange
    }
}

pub fn guest_verdict(facts: &GuestFacts) -> GuestVerdict {
    if !facts.enabled {
        return GuestVerdict::Skip(SkipReason::Disabled);
    }
    if !facts.is_guest {
        return GuestVerdict::Skip(SkipReason::NotGuest);
    }
    if !facts.local_in_play {
        return GuestVerdict::Skip(SkipReason::NotInPlay);
    }
    if facts.local_phase != Some(facts.message_phase) {
        return GuestVerdict::Skip(SkipReason::OtherPhase);
    }
    if mirror_verdict(facts.message) != MirrorVerdict::Mirror {
        return GuestVerdict::Skip(SkipReason::NotMirrored);
    }
    GuestVerdict::Raise
}
