use crate::callback_gate::{END_FOLLOW_EVENT_SUFFIX, END_FOLLOW_MASK_IDLE_MS};
use crate::coop_pause::MENU_OWN_BITS;
use crate::story_calls::is_event_callback_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndArrival {
    pub message_phase: u16,
    pub shared_phase: Option<u16>,
    pub local_phase: Option<u16>,
    pub local_in_own_talk: bool,
    pub control_mask_settled: bool,
    pub already_ran_here: bool,
    pub own_copy_of_same_event_running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndFollow {
    Mirror,
    Drop(EndDrop),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndDrop {
    OtherPhase,
    OwnTalkOpen,
    ControlMaskBusy,
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
    Drop(EndDrop),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MaskIdleTimer {
    idle_since_ms: Option<u64>,
}

impl EndDrop {
    pub const fn reason(self) -> &'static str {
        match self {
            Self::OtherPhase => "it fired in another phase",
            Self::OwnTalkOpen => "your game is in its own talk",
            Self::ControlMaskBusy => "your controls are not free yet",
            Self::AlreadyRanHere => "your game ran it already",
        }
    }

    pub const fn can_wait(self) -> bool {
        matches!(self, Self::OwnTalkOpen | Self::ControlMaskBusy)
    }
}

impl MaskIdleTimer {
    pub const fn new() -> Self {
        Self {
            idle_since_ms: None,
        }
    }

    pub fn observe(&mut self, all_slots_zero: bool, now_ms: u64) -> bool {
        if !all_slots_zero {
            self.idle_since_ms = None;
            return false;
        }
        let since = *self.idle_since_ms.get_or_insert(now_ms);
        now_ms.saturating_sub(since) >= END_FOLLOW_MASK_IDLE_MS
    }
}

pub fn wait_step(since_ms: u64, now_ms: u64, verdict: EndFollow) -> WaitStep {
    let waited_ms = now_ms.saturating_sub(since_ms);
    match verdict {
        EndFollow::Mirror => WaitStep::Run { waited_ms },
        EndFollow::Drop(reason) if !reason.can_wait() => WaitStep::Drop(reason),
        EndFollow::Drop(_) => WaitStep::KeepWaiting,
    }
}

pub fn control_slots_free(slots: [u32; 4], pause_menu_open: bool) -> bool {
    let [menu_slot, talk_slot, save_slot, message_slot] = slots;
    let menu_slot = if pause_menu_open {
        menu_slot & !MENU_OWN_BITS
    } else {
        menu_slot
    };
    menu_slot | talk_slot | save_slot | message_slot == 0
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
    if arrival.own_copy_of_same_event_running {
        return EndFollow::Mirror;
    }
    if arrival.local_in_own_talk {
        return EndFollow::Drop(EndDrop::OwnTalkOpen);
    }
    if !arrival.control_mask_settled {
        return EndFollow::Drop(EndDrop::ControlMaskBusy);
    }
    EndFollow::Mirror
}
