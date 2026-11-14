use crate::offsets::{START_TALK_NATIVE_RVA, START_TALK_UNIQUE_ACTOR_NATIVE_RVA};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TalkStartVerdict {
    RunNoInjectedEnd,
    RunSharedEventEnd,
    RunNotATalkStart,
    SkipPartnerEnd,
    SkipInsideTheHostsStepCopy,
}

impl TalkStartVerdict {
    pub const fn skips(self) -> bool {
        matches!(
            self,
            Self::SkipPartnerEnd | Self::SkipInsideTheHostsStepCopy
        )
    }

    /// The verdict for a talk start inside the guest's run of the host's copy of a step of its
    /// sequence: the talk is the host's (shown here by the mirror), this copy waits for it.
    pub const fn inside_the_hosts_step_copy(self) -> Self {
        match self {
            Self::RunNotATalkStart => Self::RunNotATalkStart,
            _ => Self::SkipInsideTheHostsStepCopy,
        }
    }

    pub const fn describe(self) -> &'static str {
        match self {
            Self::RunNoInjectedEnd => "run: no injected End window is open",
            Self::RunSharedEventEnd => {
                "run: the injected End is a shared event, both games watch it"
            }
            Self::RunNotATalkStart => "run: not a talk start native",
            Self::SkipPartnerEnd => {
                "skip: the injected End is not a shared event (own talks rule), the native returns done"
            }
            Self::SkipInsideTheHostsStepCopy => {
                "skip: the call is the host's copy of a step of its sequence, the talk is the host's"
            }
        }
    }
}

pub fn talk_start_verdict(
    native_rva: u32,
    injected_end_open: bool,
    injected_end_is_shared_event: bool,
) -> TalkStartVerdict {
    if native_rva != START_TALK_NATIVE_RVA && native_rva != START_TALK_UNIQUE_ACTOR_NATIVE_RVA {
        TalkStartVerdict::RunNotATalkStart
    } else if !injected_end_open {
        TalkStartVerdict::RunNoInjectedEnd
    } else if injected_end_is_shared_event {
        TalkStartVerdict::RunSharedEventEnd
    } else {
        TalkStartVerdict::SkipPartnerEnd
    }
}
