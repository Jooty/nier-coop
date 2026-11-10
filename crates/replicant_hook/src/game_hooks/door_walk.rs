use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use replicant_game::Position;

const WALK_MEMORY: Duration = Duration::from_secs(10);

static DOOR_USED_AT: Mutex<Option<(Instant, Position)>> = Mutex::new(None);

pub(super) fn note_door_use(position: Position) {
    *DOOR_USED_AT.lock().unwrap_or_else(PoisonError::into_inner) = Some((Instant::now(), position));
}

pub(super) fn take_recent_door_use() -> Option<Position> {
    let (at, position) = DOOR_USED_AT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()?;
    (at.elapsed() <= WALK_MEMORY).then_some(position)
}
