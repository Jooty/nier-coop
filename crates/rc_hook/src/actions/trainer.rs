use rc_game::{GameError, Replicant};

use crate::actions::action::Action;
use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;

pub(super) fn run_trainer_action(build: &VerifiedBuild, action: Action) -> Result<(), GameError> {
    let mut game = Replicant::new(InProcessMemory::new(build));
    match action {
        Action::SetInfiniteHealth { enabled } => game.set_infinite_health(enabled),
        Action::SetInfiniteMagic { enabled } => game.set_infinite_magic(enabled),
        Action::RefillHealth => {
            let max_health = game.max_health()?;
            game.set_health(max_health)
        }
        Action::RefillMagic => {
            let max_magic = game.max_magic()?;
            game.set_magic(max_magic)
        }
        Action::AddGold { amount } => {
            let gold = game.gold()?;
            game.set_gold(gold.saturating_add(amount))
        }
        Action::ChangePhase { .. } => Ok(()),
    }
}
