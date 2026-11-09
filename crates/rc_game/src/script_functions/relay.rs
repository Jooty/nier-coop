use crate::callback_gate::{is_end_callback, is_gated_callback};
use crate::script_functions::script_function;
use crate::story_calls::is_event_id_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracedRelay {
    /// Neither a shared function nor a popped event callback: it stays on this game.
    NotRelayed,
    /// A popped end, gated callback or message handler: the callback follow relays it.
    CallbackPathRelays,
    /// Sent as a shared function; the partner runs its own copy and this one only fills in.
    Send,
}

/// What the shared function relay does with a function this game traced. `popped` says the
/// engine popped it just before (a fired or queued callback, not a call from a script); every
/// popped event callback is relayed, not only the shared table's, so a step like
/// `EID_0200_a0037g_EnemyCount` reaches the partner.
pub fn traced_relay(
    phase_label: &str,
    name: &str,
    popped: bool,
    message_handler: bool,
) -> TracedRelay {
    let shared = script_function(phase_label, name).is_some();
    if !shared && !(popped && is_event_id_name(name)) {
        return TracedRelay::NotRelayed;
    }
    if popped && (is_end_callback(name) || is_gated_callback(name) || message_handler) {
        return TracedRelay::CallbackPathRelays;
    }
    TracedRelay::Send
}
