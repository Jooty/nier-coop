#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveLogReason {
    PhaseLoad,
    ControlReleased,
    TalkClosed,
}

impl MoveLogReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::PhaseLoad => "phase load",
            Self::ControlReleased => "hold released",
            Self::TalkClosed => "talk closed",
        }
    }
}

#[derive(Debug, Default)]
pub struct MoveLogTrigger {
    player: u64,
    mask: u32,
    talk_open: bool,
}

impl MoveLogTrigger {
    pub fn observe(&mut self, player: u64, mask: u32, talk_open: bool) -> Option<MoveLogReason> {
        let reason = if player == 0 {
            None
        } else if player != self.player {
            Some(MoveLogReason::PhaseLoad)
        } else if self.mask != 0 && mask == 0 {
            Some(MoveLogReason::ControlReleased)
        } else if self.talk_open && !talk_open {
            Some(MoveLogReason::TalkClosed)
        } else {
            None
        };
        self.player = player;
        self.mask = mask;
        self.talk_open = talk_open;
        reason
    }
}
