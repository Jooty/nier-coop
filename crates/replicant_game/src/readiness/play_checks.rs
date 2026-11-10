use replicant_proto::GameReadiness;

use crate::stay_together::local_phase_index;
use crate::{GameMemory, Replicant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayChecks {
    pub phase_known: bool,
    pub player_actor_spawned: bool,
    pub vitals_readable: bool,
}

pub fn classify(checks: PlayChecks) -> GameReadiness {
    if checks.phase_known && checks.player_actor_spawned && checks.vitals_readable {
        GameReadiness::InPlay
    } else {
        GameReadiness::Loading
    }
}

pub fn read_play_checks<M: GameMemory>(game: &Replicant<M>) -> PlayChecks {
    PlayChecks {
        phase_known: local_phase_index(game).is_some(),
        player_actor_spawned: game.playable_actor().is_ok(),
        vitals_readable: game.position().is_ok() && game.max_health().is_ok_and(|max| max > 0),
    }
}
