use rc_game::downed::{DOWNED_HEALTH, DownedEffect, revived_health};
use rc_game::offsets::{LIFE_FLAG_ENDURE_BIT, LIFE_FLAG_INVULNERABLE_BIT};
use rc_game::{GameError, Replicant};
use rc_proto::Message;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::downed::force_kill::force_kill;
use crate::game_hooks::set_local_health_through_game;
use crate::memory::InProcessMemory;
use crate::net::NetThread;

#[derive(Debug, Default)]
pub(super) struct Pending {
    pub leave_downed: bool,
    pub kill: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct OwnedBits {
    endure_was: bool,
    invulnerable_was: bool,
    invulnerable_set: bool,
}

pub(super) fn apply_effects(
    net: &NetThread,
    game: &mut Replicant<InProcessMemory>,
    owned: &mut Option<OwnedBits>,
    pending: &mut Pending,
    effects: &[DownedEffect],
) {
    for effect in effects {
        match effect {
            DownedEffect::Send(event) => net.send_relayed(Message::Combat(*event)),
            DownedEffect::EnterDowned => {
                if let Err(error) = enter_downed(game, owned) {
                    log_line(&format!("downed: cannot enter downed ({error})"));
                }
            }
            DownedEffect::LeaveDowned => pending.leave_downed = true,
            DownedEffect::ForceKill => pending.kill = true,
        }
        log_line(&format!("downed: {effect:?}"));
    }
}

pub(super) fn resolve_pending(
    build: &VerifiedBuild,
    game: &mut Replicant<InProcessMemory>,
    owned: &mut Option<OwnedBits>,
    pending: &mut Pending,
) {
    if pending.leave_downed {
        match leave_downed(game, owned) {
            Ok(()) => pending.leave_downed = false,
            Err(error) => log_line(&format!("downed: still downed, cannot get up ({error})")),
        }
    }
    if pending.kill && !pending.leave_downed {
        match kill_player(build, game, owned) {
            Ok(()) => pending.kill = false,
            Err(reason) => log_line(&format!("downed: kill waits ({reason})")),
        }
    }
}

pub(super) fn hold_endure(
    game: &mut Replicant<InProcessMemory>,
    owned: &mut Option<OwnedBits>,
) -> Result<(), GameError> {
    if owned.is_none() {
        *owned = Some(OwnedBits {
            endure_was: game.life_flag(LIFE_FLAG_ENDURE_BIT)?,
            invulnerable_was: game.life_flag(LIFE_FLAG_INVULNERABLE_BIT)?,
            invulnerable_set: false,
        });
    }
    game.set_life_flag(LIFE_FLAG_ENDURE_BIT, true)
}

pub(super) fn release_bits(
    game: &mut Replicant<InProcessMemory>,
    owned: &mut Option<OwnedBits>,
) -> Result<(), GameError> {
    let Some(bits) = *owned else {
        return Ok(());
    };
    game.set_life_flag(LIFE_FLAG_ENDURE_BIT, bits.endure_was)?;
    if bits.invulnerable_set {
        game.set_life_flag(LIFE_FLAG_INVULNERABLE_BIT, bits.invulnerable_was)?;
    }
    *owned = None;
    Ok(())
}

fn enter_downed(
    game: &mut Replicant<InProcessMemory>,
    owned: &mut Option<OwnedBits>,
) -> Result<(), GameError> {
    hold_endure(game, owned)?;
    game.set_life_flag(LIFE_FLAG_INVULNERABLE_BIT, true)?;
    if let Some(bits) = owned {
        bits.invulnerable_set = true;
    }
    write_health(game, DOWNED_HEALTH)
}

fn leave_downed(
    game: &mut Replicant<InProcessMemory>,
    owned: &mut Option<OwnedBits>,
) -> Result<(), GameError> {
    let max_health = game.max_health()?;
    write_health(
        game,
        revived_health(max_health)
            .min(max_health)
            .max(DOWNED_HEALTH),
    )?;
    if let Some(bits) = owned
        && bits.invulnerable_set
    {
        game.set_life_flag(LIFE_FLAG_INVULNERABLE_BIT, bits.invulnerable_was)?;
        bits.invulnerable_set = false;
    }
    Ok(())
}

fn write_health(game: &mut Replicant<InProcessMemory>, value: i32) -> Result<(), GameError> {
    if set_local_health_through_game(value) {
        return Ok(());
    }
    game.set_health(value)
}

fn kill_player(
    build: &VerifiedBuild,
    game: &mut Replicant<InProcessMemory>,
    owned: &mut Option<OwnedBits>,
) -> Result<(), String> {
    game.current_phase()
        .map_err(|_| "no readable phase, a load is probably running".to_owned())?;
    let actor = game.playable_actor().map_err(|error| error.to_string())?;
    game.set_life_flag(LIFE_FLAG_ENDURE_BIT, false)
        .and_then(|()| game.set_life_flag(LIFE_FLAG_INVULNERABLE_BIT, false))
        .map_err(|error| error.to_string())?;
    if let Some(bits) = owned {
        bits.invulnerable_set = false;
    }
    if force_kill(game.memory(), build.module_base(), actor) {
        Ok(())
    } else {
        Err("the player's vtable has no ForceKill".to_owned())
    }
}
