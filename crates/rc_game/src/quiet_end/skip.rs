use crate::offsets::{
    PAUSE_OFF_NATIVE_RVA, PAUSE_ON_NATIVE_RVA, START_TALK_NATIVE_RVA,
    START_TALK_UNIQUE_ACTOR_NATIVE_RVA,
};
use crate::pickups::InjectedEndWindow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuietEndNative {
    pub name: &'static str,
    pub rva: u32,
}

pub const QUIET_END_SKIPPED_NATIVES: [QuietEndNative; 4] = [
    QuietEndNative {
        name: "_StartTalk",
        rva: START_TALK_NATIVE_RVA,
    },
    QuietEndNative {
        name: "_StartTalkUniqueActorNotChangeEndTalkMotion",
        rva: START_TALK_UNIQUE_ACTOR_NATIVE_RVA,
    },
    QuietEndNative {
        name: "_PauseOn",
        rva: PAUSE_ON_NATIVE_RVA,
    },
    QuietEndNative {
        name: "_PauseOff",
        rva: PAUSE_OFF_NATIVE_RVA,
    },
];

pub fn skipped_native_name(
    window: &InjectedEndWindow,
    now_ms: u64,
    native_rva: u32,
) -> Option<&'static str> {
    if !window.is_open(now_ms) {
        return None;
    }
    QUIET_END_SKIPPED_NATIVES
        .iter()
        .find(|native| native.rva == native_rva)
        .map(|native| native.name)
}
