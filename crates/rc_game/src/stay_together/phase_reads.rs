use crate::memory::GameMemory;
use crate::offsets::MAX_PHASE_POINT;
use crate::stay_together::PhaseTarget;
use crate::{PhaseEntry, Replicant};

pub fn local_phase_index<M: GameMemory>(game: &Replicant<M>) -> Option<u16> {
    let name = game.current_phase().ok()?;
    u16::try_from(PhaseEntry::index_of(&name)?).ok()
}

pub fn announced_target<M: GameMemory>(game: &Replicant<M>) -> Option<PhaseTarget> {
    let phase = local_phase_index(game)?;
    let point = game
        .arrival_point()
        .ok()
        .filter(|point| (0..=MAX_PHASE_POINT).contains(point))
        .unwrap_or(0);
    Some(PhaseTarget { phase, point })
}
