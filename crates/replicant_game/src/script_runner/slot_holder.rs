use crate::fight_sync::SlotState;
use crate::script_runner::{SLOT_HOLDER_LOGGED_EVERY_MS, SLOT_HOLDER_SKIPPED_PER_FRAME_NATIVES};

/// The handler that popped last, how long it has held the script slot, the last hooked native
/// it called (only natives the hook detours are seen, per-frame ones skipped) and the last one
/// it waited in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotHolderReport {
    pub handler: String,
    pub in_slot_ms: u64,
    pub last_native: Option<(&'static str, u64)>,
    pub last_yielding_native: Option<(&'static str, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Holder {
    name: String,
    popped_ms: u64,
    ended: bool,
    last_native: Option<(&'static str, u64)>,
    last_yielding_native: Option<(&'static str, u64)>,
    next_report_after_ms: u64,
}

/// Watches the script slot's holder: each pop starts a new one, an idle slot ends it, and one
/// that stays in the slot past `SLOT_HOLDER_LOGGED_EVERY_MS` is reported once per that long.
#[derive(Debug, Clone, Default)]
pub struct SlotHolderWatch {
    holder: Option<Holder>,
}

impl SlotHolderWatch {
    pub const fn new() -> Self {
        Self { holder: None }
    }

    pub fn popped(&mut self, name: &str, now_ms: u64) {
        self.holder = Some(Holder {
            name: name.to_owned(),
            popped_ms: now_ms,
            ended: false,
            last_native: None,
            last_yielding_native: None,
            next_report_after_ms: SLOT_HOLDER_LOGGED_EVERY_MS,
        });
    }

    /// The handler that popped last while it still holds the slot.
    pub fn holding(&self) -> Option<&str> {
        self.holder
            .as_ref()
            .filter(|holder| !holder.ended)
            .map(|holder| holder.name.as_str())
    }

    /// A hooked native returned inside the script step; it belongs to the handler that popped
    /// last. A native the game calls every frame is not kept, so it never hides the others.
    pub fn native_called(&mut self, native: &'static str, now_ms: u64) {
        if SLOT_HOLDER_SKIPPED_PER_FRAME_NATIVES.contains(&native) {
            return;
        }
        if let Some(holder) = &mut self.holder {
            holder.last_native = Some((native, now_ms));
        }
    }

    /// A hooked native that holds the script step until it is done was called (once per tick
    /// while it waits); it is also the last native.
    pub fn yielding_native_called(&mut self, native: &'static str, now_ms: u64) {
        if let Some(holder) = &mut self.holder {
            holder.last_native = Some((native, now_ms));
            holder.last_yielding_native = Some((native, now_ms));
        }
    }

    /// The handler holding the slot now and for how long.
    pub fn holding_for(&self, now_ms: u64) -> Option<(&str, u64)> {
        self.holder
            .as_ref()
            .filter(|holder| !holder.ended)
            .map(|holder| {
                (
                    holder.name.as_str(),
                    now_ms.saturating_sub(holder.popped_ms),
                )
            })
    }

    /// The last native the last popped handler waited in, with how long ago.
    pub fn last_yielding_native(&self, now_ms: u64) -> Option<(&'static str, u64)> {
        let (native, at) = self.holder.as_ref()?.last_yielding_native?;
        Some((native, now_ms.saturating_sub(at)))
    }

    /// The last native the last popped handler called, with how long ago.
    pub fn last_native(&self, now_ms: u64) -> Option<(&'static str, u64)> {
        let (native, at) = self.holder.as_ref()?.last_native?;
        Some((native, now_ms.saturating_sub(at)))
    }

    /// Once per tick with the script slot's state; returns the report when one is due.
    pub fn tick(&mut self, slot: SlotState, now_ms: u64) -> Option<SlotHolderReport> {
        let holder = self.holder.as_mut()?;
        if slot == SlotState::Idle {
            holder.ended = true;
        }
        if holder.ended {
            return None;
        }
        let in_slot_ms = now_ms.saturating_sub(holder.popped_ms);
        if in_slot_ms <= holder.next_report_after_ms {
            return None;
        }
        holder.next_report_after_ms = in_slot_ms + SLOT_HOLDER_LOGGED_EVERY_MS;
        Some(SlotHolderReport {
            handler: holder.name.clone(),
            in_slot_ms,
            last_native: holder
                .last_native
                .map(|(native, at)| (native, now_ms.saturating_sub(at))),
            last_yielding_native: holder
                .last_yielding_native
                .map(|(native, at)| (native, now_ms.saturating_sub(at))),
        })
    }
}
