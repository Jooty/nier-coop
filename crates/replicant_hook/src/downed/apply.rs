use replicant_game::downed::{
    BACKUP_WAITS_FOR_PLAY_SECONDS, DOWNED_HEALTH, DownedEffect, DownedRules,
    HEAL_WATCH_GIVES_UP_SECONDS, REWIND_WAITS_FOR_PLAY_SECONDS, revived_health,
};
use replicant_game::offsets::{LIFE_FLAG_ENDURE_BIT, LIFE_FLAG_INVULNERABLE_BIT};
use replicant_game::{GameError, Replicant};
use replicant_proto::Message;

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
    pub backup_waited: Option<f32>,
    pub rewind_waited: Option<f32>,
    pub heal_watch: Option<HealWatch>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct HealWatch {
    waited_seconds: f32,
    saw_load: bool,
}

#[derive(Debug, Default)]
pub(super) struct RewindStep {
    pub rewound: bool,
    pub effects: Vec<DownedEffect>,
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
            DownedEffect::TakeBackup => pending.backup_waited = Some(0.0),
            DownedEffect::Rewind => pending.rewind_waited = Some(0.0),
            DownedEffect::DropRewind(reason) => {
                log_line(&format!("rewind: host game over ignored ({reason:?})"));
            }
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
            Ok(()) => {
                pending.kill = false;
                crate::rewind::arm_game_over();
            }
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

pub(super) fn resolve_backup(
    build: &VerifiedBuild,
    game: &Replicant<InProcessMemory>,
    rules: &mut DownedRules,
    pending: &mut Pending,
    dt_seconds: f32,
) {
    let Some(waited) = pending.backup_waited.as_mut() else {
        return;
    };
    *waited += dt_seconds;
    if game.current_phase().is_ok() {
        pending.backup_waited = None;
        if crate::rewind::take_backup(build) {
            rules.backup_taken();
            log_line("rewind: guest took its own checkpoint with the host's");
        } else {
            log_line("rewind: guest cannot take a checkpoint, the backup hook is missing");
        }
    } else if *waited >= BACKUP_WAITS_FOR_PLAY_SECONDS {
        pending.backup_waited = None;
        log_line("rewind: guest dropped the host's checkpoint, it never got to play");
    }
}

pub(super) fn resolve_rewind(
    build: &VerifiedBuild,
    game: &mut Replicant<InProcessMemory>,
    owned: &mut Option<OwnedBits>,
    rules: &mut DownedRules,
    pending: &mut Pending,
    dt_seconds: f32,
) -> RewindStep {
    let Some(waited) = pending.rewind_waited.as_mut() else {
        return RewindStep::default();
    };
    *waited += dt_seconds;
    if game.current_phase().is_err() {
        if *waited < REWIND_WAITS_FOR_PLAY_SECONDS {
            return RewindStep::default();
        }
        pending.rewind_waited = None;
        log_line("rewind: guest dropped the host's game over, it never got to play");
        return RewindStep {
            rewound: false,
            effects: rules.rewind_dropped(),
        };
    }
    pending.rewind_waited = None;
    if let Err(error) = release_bits(game, owned) {
        log_line(&format!("rewind: life flags not released ({error})"));
    }
    pending.kill = false;
    pending.leave_downed = false;
    if !crate::rewind::rewind_now(build) {
        log_line("rewind: guest cannot go back, the restore hook is missing");
        return RewindStep {
            rewound: false,
            effects: rules.rewind_dropped(),
        };
    }
    heal_fully(game);
    pending.heal_watch = Some(HealWatch::default());
    rules.rewind_done();
    crate::rewind::note_rewound();
    log_line("rewind: guest restored its checkpoint and restarted the phase");
    RewindStep {
        rewound: true,
        effects: Vec::new(),
    }
}

pub(super) fn heal_after_load(
    game: &mut Replicant<InProcessMemory>,
    pending: &mut Pending,
    dt_seconds: f32,
) {
    let Some(watch) = pending.heal_watch.as_mut() else {
        return;
    };
    watch.waited_seconds += dt_seconds;
    if game.current_phase().is_err() {
        watch.saw_load = true;
    } else if watch.saw_load {
        heal_fully(game);
        pending.heal_watch = None;
        return;
    }
    if watch.waited_seconds >= HEAL_WATCH_GIVES_UP_SECONDS {
        pending.heal_watch = None;
    }
}

fn heal_fully(game: &mut Replicant<InProcessMemory>) {
    match game.max_health().and_then(|max| write_health(game, max)) {
        Ok(()) => log_line("rewind: guest healed to full health"),
        Err(error) => log_line(&format!("rewind: guest not healed ({error})")),
    }
}
