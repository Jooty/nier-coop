use std::ffi::CString;

use rc_game::offsets::{
    ACTOR_MODEL_PACK_OFFSET, ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET,
    PUPPET_UNIQUE_ID_BASE,
};
use rc_game::puppet::{Affine, book_hover_offset, book_pack_name};
use rc_game::{GameMemory, Replicant};
use rc_proto::PartnerBody;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::puppet::actor::Puppet;
use crate::puppet::error::PuppetError;
use crate::puppet::game_calls::GameCalls;
use crate::puppet::pack_guard::{ensure_pack_guard, guard, still_guarded, unguard};

const BOOK_UNIQUE_ID_OFFSET: u32 = 0x9000;

#[derive(Debug)]
struct ShownBook {
    body: PartnerBody,
    name: CString,
    actor: Puppet,
}

#[derive(Debug, Default)]
pub(super) struct PuppetBook {
    shown: Option<ShownBook>,
    requested: Option<CString>,
    last_note: Option<String>,
}

impl PuppetBook {
    pub(super) fn drive(
        &mut self,
        build: &VerifiedBuild,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
        puppet_id: u32,
        (body, now_ms): (PartnerBody, u64),
    ) {
        self.forget_if_gone(game);
        if self.shown.as_ref().is_some_and(|shown| shown.body != body) {
            self.despawn(game, true);
        }
        if self.shown.is_none() {
            self.try_show(build, game, puppet, puppet_id, body);
        }
        if let Err(error) = self.place(game, puppet, now_ms) {
            self.note(format!("puppet book: cannot place ({error})"));
        }
    }

    pub(super) fn actor_address(&self) -> Option<u64> {
        self.shown.as_ref().map(|shown| shown.actor.actor())
    }

    pub(super) fn move_to_world_position(
        &self,
        game: &Replicant<InProcessMemory>,
        position: [f32; 3],
    ) -> Option<u64> {
        let shown = self.shown.as_ref()?;
        let matrix = Affine::translation(position).to_matrix_bytes();
        shown.actor.set_matrix(game, &matrix).ok()?;
        Some(shown.actor.actor())
    }

    pub(super) fn despawn(&mut self, game: &Replicant<InProcessMemory>, unchecked_is_live: bool) {
        let Some(shown) = self.shown.take() else {
            return;
        };
        if !still_guarded(shown.actor.actor()) {
            return;
        }
        unguard(shown.actor.actor());
        let live = if shown.actor.liveness_is_trusted() {
            shown.actor.is_live(game)
        } else {
            unchecked_is_live
        };
        if live {
            shown.actor.despawn_detached(game);
        }
        log_line(&format!("puppet book: {:?} removed", shown.name));
    }

    fn forget_if_gone(&mut self, game: &Replicant<InProcessMemory>) {
        let Some(shown) = &self.shown else {
            return;
        };
        let released = !still_guarded(shown.actor.actor());
        let vanished = shown.actor.liveness_is_trusted() && !shown.actor.is_live(game);
        if released || vanished {
            log_line(&format!(
                "puppet book: {:?} is gone ({}), will show it again when its pack is back",
                shown.name,
                if released {
                    "pack released"
                } else {
                    "actor removed"
                }
            ));
            if vanished {
                unguard(shown.actor.actor());
            }
            self.shown = None;
            self.requested = None;
        }
    }

    fn try_show(
        &mut self,
        build: &VerifiedBuild,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
        puppet_id: u32,
        body: PartnerBody,
    ) {
        let Ok(name) = CString::new(book_pack_name(body)) else {
            return;
        };
        if !ensure_pack_guard(build) {
            self.note("puppet book: release guard missing, book stays hidden".to_owned());
            return;
        }
        let calls = GameCalls::new(build);
        let pack = calls.find_model_pack(&name);
        if pack == 0 {
            if self.requested.as_ref() != Some(&name) {
                let slot = calls.request_model_pack(&name);
                log_line(&format!(
                    "puppet book: loading {name:?} for body {body:?} (slot {slot:#x})"
                ));
                self.requested = Some(name);
            }
            return;
        }
        let unique_id = PUPPET_UNIQUE_ID_BASE
            .saturating_add_unsigned(BOOK_UNIQUE_ID_OFFSET)
            .saturating_add_unsigned(puppet_id);
        let matrix = match game.memory().read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(
            puppet.actor() + ACTOR_TRANSFORM_BLOCK_OFFSET,
        ) {
            Ok(block) => Affine::from_block(&block).to_matrix_bytes(),
            Err(error) => {
                self.note(format!(
                    "puppet book: puppet transform unreadable ({error})"
                ));
                return;
            }
        };
        let actor = match Puppet::spawn_model(build, &name, unique_id, &matrix) {
            Ok(actor) => actor,
            Err(error) => {
                self.note(format!("puppet book: cannot spawn {name:?} ({error})"));
                return;
            }
        };
        let worn = game
            .memory()
            .read_u64(actor.actor() + ACTOR_MODEL_PACK_OFFSET)
            .unwrap_or(0);
        guard(actor.actor(), if worn == 0 { pack } else { worn });
        log_line(&format!(
            "puppet book: showing {name:?} for body {body:?} as actor {:#x}, pack {pack:#x}, worn {worn:#x}",
            actor.actor()
        ));
        self.shown = Some(ShownBook { body, name, actor });
        self.last_note = None;
    }

    fn place(
        &self,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
        now_ms: u64,
    ) -> Result<(), PuppetError> {
        let Some(shown) = &self.shown else {
            return Ok(());
        };
        let root = game.memory().read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(
            puppet.actor() + ACTOR_TRANSFORM_BLOCK_OFFSET,
        )?;
        let world = Affine::from_block(&root).then(&Affine::translation(book_hover_offset(now_ms)));
        shown.actor.set_matrix(game, &world.to_matrix_bytes())
    }

    fn note(&mut self, message: String) {
        if self.last_note.as_deref() != Some(message.as_str()) {
            log_line(&message);
            self.last_note = Some(message);
        }
    }
}
