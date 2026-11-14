use crate::event_follow::EventId;

/// What the hook calls in the game on the game thread, as ours, in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCall {
    /// The play request held at the script's call is made now.
    ReleaseHeldPlay(EventId),
    Load(EventId),
    Play(EventId),
    Release(EventId),
    /// The game's skip function, which skips every playing event of this copy.
    Skip,
    /// The event clock of this copy is stopped (the partner opened its event pause).
    PauseClock,
    ResumeClock,
}
