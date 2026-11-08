use crate::callback_gate::{END_FOLLOW_EVENT_SUFFIX, END_FOLLOW_HOST_FIRED_WINDOW_MS, GateRole};
use crate::story_calls::is_event_callback_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndArrival {
    pub role: GateRole,
    pub message_phase: u16,
    pub shared_phase: Option<u16>,
    pub local_phase: Option<u16>,
    pub menu_open: bool,
    pub host_in_own_talk: bool,
    pub host_fired_it_ms_ago: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndFollow {
    Mirror,
    Drop(EndDrop),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndDrop {
    NotTheHost,
    OtherPhase,
    MenuOpen,
    HostInOwnTalk,
    HostFiredItItself,
}

impl EndDrop {
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NotTheHost => "only the host runs the guest's ends",
            Self::OtherPhase => "it fired in another phase",
            Self::MenuOpen => "a menu is open on the host",
            Self::HostInOwnTalk => "the host is in its own talk",
            Self::HostFiredItItself => "the host's game fired it already",
        }
    }
}

pub fn is_end_callback(name: &str) -> bool {
    is_event_callback_name(name)
        && name
            .trim_end_matches(|character: char| character.is_ascii_digit())
            .ends_with(END_FOLLOW_EVENT_SUFFIX)
}

pub fn end_follow(arrival: &EndArrival) -> EndFollow {
    if arrival.role != GateRole::Host {
        return EndFollow::Drop(EndDrop::NotTheHost);
    }
    let same_phase = arrival.shared_phase == Some(arrival.message_phase)
        && arrival.local_phase == Some(arrival.message_phase);
    if !same_phase {
        return EndFollow::Drop(EndDrop::OtherPhase);
    }
    if arrival.menu_open {
        return EndFollow::Drop(EndDrop::MenuOpen);
    }
    if arrival.host_in_own_talk {
        return EndFollow::Drop(EndDrop::HostInOwnTalk);
    }
    if arrival
        .host_fired_it_ms_ago
        .is_some_and(|ago| ago <= END_FOLLOW_HOST_FIRED_WINDOW_MS)
    {
        return EndFollow::Drop(EndDrop::HostFiredItItself);
    }
    EndFollow::Mirror
}
