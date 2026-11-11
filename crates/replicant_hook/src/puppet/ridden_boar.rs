use std::ffi::CString;

use replicant_game::game_closing::actors_to_destroy;
use replicant_game::npcs::read_npc_body_layer;
use replicant_game::offsets::{
    ACTOR_MODEL_PACK_OFFSET, BODY_MOTION_LAYER, PUPPET_UNIQUE_ID_BASE, RIDDEN_BOAR_MODEL_PACK_NAME,
};
use replicant_game::puppet::{
    BoarPackSettle, BoarStep, MotionStep, PuppetPose, RespawnGate, boar_pose_under_rider,
    boar_step, follow_step, frame_to_seek, seek_layer_frame,
};
use replicant_game::{GameMemory, Replicant};
use replicant_proto::{MotionState, RiddenBoar};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;
use crate::puppet::actor::Puppet;
use crate::puppet::game_calls::GameCalls;
use crate::puppet::pack_guard::{
    ensure_pack_guard, game_is_closing, guard, has_free_slot, still_guarded, unguard,
};

const BOAR_UNIQUE_ID_OFFSET: u32 = 0xB000;

#[derive(Debug)]
struct ShownBoar {
    actor: Puppet,
    last_started: Option<i32>,
}

#[derive(Debug, Default)]
pub(super) struct PuppetBoar {
    shown: Option<ShownBoar>,
    settle: BoarPackSettle,
    respawn_gate: RespawnGate,
    last_note: Option<String>,
}

impl PuppetBoar {
    pub(super) fn drive(
        &mut self,
        build: &VerifiedBuild,
        game: &Replicant<InProcessMemory>,
        rider: PuppetPose,
        puppet_id: u32,
        boar: Option<RiddenBoar>,
    ) {
        self.forget_if_gone(game);
        let Ok(name) = CString::new(RIDDEN_BOAR_MODEL_PACK_NAME) else {
            return;
        };
        let now_ms = net_clock_ms();
        let (mut pack, mut settled) = (0, false);
        // A shown boar needs no lookup, and its settle time is kept for the next ride.
        if self.shown.is_none() && self.settle.lookup_due(boar.is_some(), now_ms) {
            pack = GameCalls::new(build).find_model_pack(&name);
            let loading = game.playable_actor().is_err() || game.current_phase().is_err();
            settled = self.settle.settled(pack != 0, loading, now_ms);
        }
        let step = boar_step(
            boar.is_some(),
            self.shown.is_some(),
            settled,
            game_is_closing(),
        );
        let Some(boar) = boar else {
            if step == BoarStep::Remove {
                self.despawn(game, true, "the partner no longer rides");
            }
            return;
        };
        let pose = boar_pose_under_rider(rider, &boar);
        match step {
            BoarStep::Make => self.try_show(build, game, puppet_id, (pack, pose)),
            BoarStep::Wait if pack == 0 => self.note(format!(
                "ridden boar: the partner rides, but {name:?} is not loaded here; they are shown without a boar"
            )),
            _ => {}
        }
        self.place(build, game, pose, boar.motion);
    }

    pub(super) fn actor_address(&self) -> Option<u64> {
        self.shown.as_ref().map(|shown| shown.actor.actor())
    }

    pub(super) fn despawn(
        &mut self,
        game: &Replicant<InProcessMemory>,
        unchecked_is_live: bool,
        why: &str,
    ) {
        let Some(shown) = self.shown.take() else {
            return;
        };
        let actor = shown.actor.actor();
        if !still_guarded(actor) {
            return;
        }
        unguard(actor);
        let live = if shown.actor.liveness_is_trusted() {
            shown.actor.is_live(game)
        } else {
            unchecked_is_live
        };
        if live && !actors_to_destroy(game_is_closing(), vec![actor]).is_empty() {
            shown.actor.despawn_detached(game);
        }
        log_line(&format!("ridden boar: {actor:#x} removed ({why})"));
    }

    fn forget_if_gone(&mut self, game: &Replicant<InProcessMemory>) {
        let Some(shown) = &self.shown else {
            return;
        };
        let actor = shown.actor.actor();
        let released = !still_guarded(actor);
        let vanished = shown.actor.liveness_is_trusted() && !shown.actor.is_live(game);
        if released || vanished {
            log_line(&format!(
                "ridden boar: {actor:#x} is gone ({}), it is shown again when its model is back",
                if released {
                    "model released"
                } else {
                    "actor removed"
                }
            ));
            if vanished {
                unguard(actor);
            }
            self.shown = None;
        }
    }

    fn try_show(
        &mut self,
        build: &VerifiedBuild,
        game: &Replicant<InProcessMemory>,
        puppet_id: u32,
        (pack, pose): (u64, PuppetPose),
    ) {
        let Ok(name) = CString::new(RIDDEN_BOAR_MODEL_PACK_NAME) else {
            return;
        };
        if !ensure_pack_guard(build) {
            self.note("ridden boar: release guard missing, no boar is shown".to_owned());
            return;
        }
        if !self.respawn_gate.may_spawn(pack != 0, net_clock_ms()) {
            return;
        }
        if !has_free_slot() {
            self.note("ridden boar: no free pack guard slot, no boar is shown".to_owned());
            return;
        }
        let unique_id = PUPPET_UNIQUE_ID_BASE
            .saturating_add_unsigned(BOAR_UNIQUE_ID_OFFSET)
            .saturating_add_unsigned(puppet_id);
        let actor = match Puppet::spawn_model_by_class_id_only(
            build,
            &name,
            unique_id,
            &pose.creation_matrix(),
        ) {
            Ok(actor) => actor,
            Err(error) => {
                self.note(format!("ridden boar: cannot make {name:?} ({error})"));
                return;
            }
        };
        let worn = game
            .memory()
            .read_u64(actor.actor() + ACTOR_MODEL_PACK_OFFSET)
            .unwrap_or(0);
        // An actor that wears no pack, or one nothing guards, is not kept: playing a motion on
        // it or outliving its pack is what the guard exists to prevent.
        if worn == 0 || worn != pack || !guard(actor.actor(), worn) {
            self.note(format!(
                "ridden boar: {name:?} came out wearing pack {worn:#x} (found {pack:#x}), removed again"
            ));
            actor.despawn();
            return;
        }
        log_line(&format!(
            "ridden boar: showing {name:?} under the partner as actor {:#x}, pack {pack:#x}, worn {worn:#x}",
            actor.actor()
        ));
        self.shown = Some(ShownBoar {
            actor,
            last_started: None,
        });
        self.last_note = None;
    }

    fn place(
        &mut self,
        build: &VerifiedBuild,
        game: &Replicant<InProcessMemory>,
        pose: PuppetPose,
        motion: Option<MotionState>,
    ) {
        let Some(shown) = &mut self.shown else {
            return;
        };
        let placed = shown.actor.set_matrix(game, &pose.creation_matrix());
        if let Some(motion) = motion {
            follow_motion(build, game, shown, motion);
        }
        if let Err(error) = placed {
            self.note(format!("ridden boar: cannot place ({error})"));
        }
    }

    fn note(&mut self, message: String) {
        if self.last_note.as_deref() != Some(message.as_str()) {
            log_line(&message);
            self.last_note = Some(message);
        }
    }
}

fn follow_motion(
    build: &VerifiedBuild,
    game: &Replicant<InProcessMemory>,
    shown: &mut ShownBoar,
    target: MotionState,
) {
    let actor = shown.actor.actor();
    // PlayMotion asks the worn pack first, so it is never called on an actor without one.
    let wears_pack = game
        .memory()
        .read_u64(actor + ACTOR_MODEL_PACK_OFFSET)
        .is_ok_and(|pack| pack != 0);
    if !wears_pack || !still_guarded(actor) {
        return;
    }
    let index = i32::from(target.index);
    let mut memory = InProcessMemory::new(build);
    let layer = read_npc_body_layer(&memory, actor).ok().flatten();
    match follow_step(layer.map(|layer| layer.motion), shown.last_started, index) {
        MotionStep::Start { blend } => {
            shown.actor.play_motion(
                BODY_MOTION_LAYER,
                index,
                blend,
                target.looping,
                target.speed,
                target.frame,
            );
            log_line(&format!(
                "ridden boar: {actor:#x} plays motion {index} from frame {:.1} (looping {}, was {})",
                target.frame,
                target.looping,
                layer.map_or(-1, |layer| layer.motion.index)
            ));
            shown.last_started = Some(index);
        }
        MotionStep::Keep => {
            if let Some(layer) = layer.filter(|layer| layer.motion.index == index)
                && let Some(frame) = frame_to_seek(
                    layer.motion.frame,
                    target.frame,
                    layer.frame_count,
                    layer.motion.looping,
                )
            {
                let _ = seek_layer_frame(&mut memory, &layer, frame);
            }
        }
    }
}
