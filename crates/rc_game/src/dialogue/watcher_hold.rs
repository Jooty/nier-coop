use crate::dialogue::WATCHER_HOLD_MASK;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatcherHoldAction {
    Hold,
    Release,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WatcherHoldInput {
    pub mirror_showing: bool,
    pub slot_one_mask: u32,
    /// Game code other than the hold wrote control mask slot 1 since the last tick.
    pub taken_over: bool,
}

/// Keeps the player still while the window only mirrors the partner's talk.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WatcherHold {
    holding: bool,
}

impl WatcherHold {
    pub const fn new() -> Self {
        Self { holding: false }
    }

    pub fn is_holding(&self) -> bool {
        self.holding
    }

    pub fn step(&mut self, input: WatcherHoldInput) -> Option<WatcherHoldAction> {
        if self.holding && input.taken_over {
            self.holding = false;
            return None;
        }
        if self.holding {
            if input.mirror_showing {
                return None;
            }
            self.holding = false;
            return (input.slot_one_mask == WATCHER_HOLD_MASK)
                .then_some(WatcherHoldAction::Release);
        }
        if input.mirror_showing && input.slot_one_mask == 0 {
            self.holding = true;
            return Some(WatcherHoldAction::Hold);
        }
        None
    }

    pub fn abandon(&mut self) {
        self.holding = false;
    }

    pub fn release_on_link_loss(&mut self, slot_one_mask: u32) -> Option<WatcherHoldAction> {
        self.step(WatcherHoldInput {
            mirror_showing: false,
            slot_one_mask,
            taken_over: false,
        })
    }
}

/// Whether the open talk window is this player's own: the game wrote control mask slot 1 for
/// it, not the mirror's own window calls, and the watcher hold is not what set the mask.
pub fn own_talk_started(
    slot_one_mask: u32,
    mask_written_by_mirror: bool,
    holding: bool,
    taken_over: bool,
) -> bool {
    slot_one_mask != 0 && !mask_written_by_mirror && (!holding || taken_over)
}
