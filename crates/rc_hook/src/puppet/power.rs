use std::collections::BTreeMap;
use std::ffi::CString;
use std::sync::{LazyLock, Mutex, PoisonError};

use rc_game::offsets::{ACTOR_MODEL_PACK_OFFSET, PUPPET_UNIQUE_ID_BASE};
use rc_game::power_models::{PowerModelTable, power_model_pack_name};
use rc_game::puppet::{PuppetPose, RespawnGate};
use rc_game::{GameMemory, Replicant};
use rc_proto::{Message, PowerModelState};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::puppet::actor::Puppet;
use crate::puppet::game_calls::GameCalls;
use crate::puppet::pack_guard::{ensure_pack_guard, guard, has_free_slot, still_guarded, unguard};

const POWER_UNIQUE_ID_OFFSET: u32 = 0xA000;

static POWERS: LazyLock<Mutex<PartnerPowers>> = LazyLock::new(Mutex::default);

#[derive(Debug)]
struct ShownPower {
    name: CString,
    actor: Puppet,
}

#[derive(Debug, Default)]
struct PartnerPowers {
    table: PowerModelTable,
    latest: BTreeMap<u16, PowerModelState>,
    shown: BTreeMap<u16, ShownPower>,
    gates: BTreeMap<u16, RespawnGate>,
    requested: Vec<CString>,
    last_note: Option<String>,
}

impl PartnerPowers {
    fn clear(&mut self, game: &Replicant<InProcessMemory>) {
        for instance in self.table.take_all() {
            self.remove(game, instance, "partner not in play");
        }
        let leftovers: Vec<u16> = self.shown.keys().copied().collect();
        for instance in leftovers {
            self.remove(game, instance, "partner not in play");
        }
    }

    fn remove(&mut self, game: &Replicant<InProcessMemory>, instance: u16, why: &str) {
        self.latest.remove(&instance);
        self.gates.remove(&instance);
        let Some(shown) = self.shown.remove(&instance) else {
            return;
        };
        if still_guarded(shown.actor.actor()) {
            unguard(shown.actor.actor());
            let live = !shown.actor.liveness_is_trusted() || shown.actor.is_live(game);
            if live {
                shown.actor.despawn_detached(game);
            }
        }
        log_line(&format!(
            "puppet power: {:?} #{instance} removed ({why})",
            shown.name
        ));
    }

    fn forget_vanished(&mut self, game: &Replicant<InProcessMemory>) {
        let gone: Vec<u16> = self
            .shown
            .iter()
            .filter(|(_, shown)| {
                !still_guarded(shown.actor.actor())
                    || (shown.actor.liveness_is_trusted() && !shown.actor.is_live(game))
            })
            .map(|(instance, _)| *instance)
            .collect();
        for instance in gone {
            if let Some(shown) = self.shown.remove(&instance) {
                unguard(shown.actor.actor());
                self.requested.retain(|requested| *requested != shown.name);
                log_line(&format!(
                    "puppet power: {:?} #{instance} is gone, will show it again when its pack is back",
                    shown.name
                ));
            }
        }
    }

    fn try_show(
        &mut self,
        build: &VerifiedBuild,
        game: &Replicant<InProcessMemory>,
        state: PowerModelState,
        now_ms: u64,
    ) {
        let Ok(name) = CString::new(power_model_pack_name(state.kind)) else {
            return;
        };
        if !ensure_pack_guard(build) {
            self.note("puppet power: release guard missing, power models stay hidden");
            return;
        }
        let calls = GameCalls::new(build);
        let pack = calls.find_model_pack(&name);
        if pack != 0 {
            self.requested.retain(|requested| *requested != name);
        }
        if pack == 0 {
            if !self.requested.contains(&name) {
                let slot = calls.request_model_pack(&name);
                log_line(&format!("puppet power: loading {name:?} (slot {slot:#x})"));
                self.requested.push(name);
            }
            return;
        }
        let gate = self.gates.entry(state.instance).or_default();
        if !gate.may_spawn(true, now_ms) {
            return;
        }
        if !has_free_slot() {
            self.note("puppet power: no free pack guard slot, power model stays hidden");
            return;
        }
        let unique_id = PUPPET_UNIQUE_ID_BASE
            .saturating_add_unsigned(POWER_UNIQUE_ID_OFFSET)
            .saturating_add_unsigned(u32::from(state.instance));
        let matrix = pose_of(state).creation_matrix();
        let actor = match Puppet::spawn_model(build, &name, unique_id, &matrix) {
            Ok(actor) => actor,
            Err(error) => {
                self.note(&format!("puppet power: cannot spawn {name:?} ({error})"));
                return;
            }
        };
        let worn = game
            .memory()
            .read_u64(actor.actor() + ACTOR_MODEL_PACK_OFFSET)
            .unwrap_or(0);
        guard(actor.actor(), if worn == 0 { pack } else { worn });
        log_line(&format!(
            "puppet power: showing {name:?} #{} as actor {:#x} at {:?}, pack {pack:#x}, worn {worn:#x}",
            state.instance,
            actor.actor(),
            state.position
        ));
        self.shown
            .insert(state.instance, ShownPower { name, actor });
    }

    fn place(&self, game: &Replicant<InProcessMemory>, state: PowerModelState) {
        let Some(shown) = self.shown.get(&state.instance) else {
            return;
        };
        if let Err(error) = shown
            .actor
            .set_matrix(game, &pose_of(state).creation_matrix())
        {
            log_line(&format!("puppet power: cannot place ({error})"));
        }
    }

    fn note(&mut self, message: &str) {
        if self.last_note.as_deref() != Some(message) {
            log_line(message);
            self.last_note = Some(message.to_owned());
        }
    }
}

pub fn tick_partner_powers(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let arrived = net.take_relayed(|message| matches!(message, Message::PowerModel(_)));
    let game = Replicant::new(InProcessMemory::new(build));
    let mut powers = POWERS.lock().unwrap_or_else(PoisonError::into_inner);
    if !net.view().partner_in_play() {
        powers.clear(&game);
        return;
    }
    let now_ms = net_clock_ms();
    for message in arrived {
        let Message::PowerModel(state) = message else {
            continue;
        };
        if powers.table.note(state.instance, now_ms) {
            powers.latest.insert(state.instance, state);
        } else {
            powers.note("puppet power: too many partner models at once, one is skipped");
        }
    }
    for instance in powers.table.take_stale(now_ms) {
        powers.remove(&game, instance, "no longer sent");
    }
    powers.forget_vanished(&game);
    let wanted: Vec<PowerModelState> = powers.latest.values().copied().collect();
    for state in wanted {
        if !powers.shown.contains_key(&state.instance) {
            powers.try_show(build, &game, state, now_ms);
        }
        powers.place(&game, state);
    }
}

pub fn power_actor_addresses() -> Vec<u64> {
    let Ok(powers) = POWERS.try_lock() else {
        return Vec::new();
    };
    powers
        .shown
        .values()
        .map(|shown| shown.actor.actor())
        .collect()
}

fn pose_of(state: PowerModelState) -> PuppetPose {
    PuppetPose {
        position: rc_game::Position {
            x: state.position[0],
            y: state.position[1],
            z: state.position[2],
        },
        yaw: state.yaw_radians,
    }
}
