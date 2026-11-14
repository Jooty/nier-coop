use std::sync::PoisonError;

use replicant_game::rule_rooms::{RULE_ROOMS, kept_message_line};
use replicant_game::{GameMemory, Replicant};

use crate::diagnostics::log_line;
use crate::rule_rooms::state::WATCH;

/// On the host: true when `message` breaks the rule of the room this game is in, so it stays out
/// of the mirror: a break sends back only the player whose game raised it.
pub(crate) fn keeps_message_here<M: GameMemory>(game: &Replicant<M>, message: u32) -> bool {
    if !RULE_ROOMS
        .iter()
        .any(|room| room.has_break_message(message))
    {
        return false;
    }
    let Ok(pack) = game.current_phase() else {
        return false;
    };
    let kept = WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .keeps_message_here(&pack, message);
    if let Some(room) = kept {
        log_line(&kept_message_line(room, message));
    }
    kept.is_some()
}
