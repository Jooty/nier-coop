use crate::callback_gate::{END_FOLLOW_EVENT_SUFFIX, END_FOLLOW_WAIT_CAP_MS};
use crate::story_calls::is_event_callback_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndArrival {
    pub message_phase: u16,
    pub shared_phase: Option<u16>,
    pub local_phase: Option<u16>,
    pub menu_open: bool,
    pub local_in_own_talk: bool,
    pub already_ran_here: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndFollow {
    Mirror,
    Drop(EndDrop),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndDrop {
    OtherPhase,
    MenuOpen,
    OwnTalkOpen,
    AlreadyRanHere,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingEnd {
    pub name: String,
    pub phase: u16,
    pub since_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitStep {
    Run { waited_ms: u64 },
    KeepWaiting,
    GiveUp { waited_ms: u64 },
    Drop(EndDrop),
}

impl EndDrop {
    pub const fn reason(self) -> &'static str {
        match self {
            Self::OtherPhase => "it fired in another phase",
            Self::MenuOpen => "a menu is open on the host",
            Self::OwnTalkOpen => "your game is in its own talk",
            Self::AlreadyRanHere => "your game ran it already",
        }
    }

    pub const fn can_wait(self) -> bool {
        matches!(self, Self::MenuOpen | Self::OwnTalkOpen)
    }
}

pub fn wait_step(since_ms: u64, now_ms: u64, verdict: EndFollow) -> WaitStep {
    let waited_ms = now_ms.saturating_sub(since_ms);
    match verdict {
        EndFollow::Mirror => WaitStep::Run { waited_ms },
        EndFollow::Drop(reason) if !reason.can_wait() => WaitStep::Drop(reason),
        EndFollow::Drop(_) if waited_ms >= END_FOLLOW_WAIT_CAP_MS => WaitStep::GiveUp { waited_ms },
        EndFollow::Drop(_) => WaitStep::KeepWaiting,
    }
}

pub fn is_end_callback(name: &str) -> bool {
    is_event_callback_name(name)
        && name
            .trim_end_matches(|character: char| character.is_ascii_digit())
            .ends_with(END_FOLLOW_EVENT_SUFFIX)
}

pub fn end_follow(arrival: &EndArrival) -> EndFollow {
    let same_phase = arrival.shared_phase == Some(arrival.message_phase)
        && arrival.local_phase == Some(arrival.message_phase);
    if !same_phase {
        return EndFollow::Drop(EndDrop::OtherPhase);
    }
    if arrival.already_ran_here {
        return EndFollow::Drop(EndDrop::AlreadyRanHere);
    }
    if arrival.menu_open {
        return EndFollow::Drop(EndDrop::MenuOpen);
    }
    if arrival.local_in_own_talk {
        return EndFollow::Drop(EndDrop::OwnTalkOpen);
    }
    EndFollow::Mirror
}
