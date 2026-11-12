use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Instant;

use automata_game::mode::SessionMode;
use automata_game::partner::{
    ExtraBodyRoster, ExtraBodyStep, OtherPlayer, PartnerDrive, RosterReadings,
    SPAWN_RETRY_INTERVAL_MS, partner_positions_apply,
};
use automata_game::start::is_in_play;
use automata_proto::PlayerState;
use nc_hook::diagnostics::log_line;

use crate::extra_bodies::EXTRA_BODY_LOG_OWNER_PREFIX;
use crate::extra_bodies::body_maker::{make_body, remove_body};
use crate::extra_bodies::data_requests::DataRequests;
use crate::extra_bodies::session_players::{FurtherPlayer, other_players_of, session_mode_of};
use crate::game_calls::GameCalls;
use crate::game_hooks::release_body;
use crate::net::handle;
use crate::partner_driver::BodyDrive;

static EXTRA_BODIES: Mutex<Option<ExtraBodies>> = Mutex::new(None);

struct ExtraBodies {
    calls: GameCalls,
    spawn_is_guarded: bool,
    started: Instant,
    roster: ExtraBodyRoster,
    drives: BTreeMap<u8, BodyDrive>,
    failed_spawn_at_ms: BTreeMap<u8, u64>,
    reads: DataRequests,
}

impl ExtraBodies {
    fn tick(&mut self, mode: SessionMode, players: &[FurtherPlayer]) {
        let now_ms = self.started.elapsed().as_millis() as u64;
        let calls = self.calls;
        let hero = calls.current_hero();
        let play = calls.read_play(hero != 0);
        let others: Vec<OtherPlayer> = players.iter().map(|player| player.roster).collect();
        let readings = RosterReadings {
            mode,
            scene_in_play: is_in_play(&play),
            phase_id: play.phase_id,
            others: &others,
        };
        self.reads.forget_other_places(play.phase_id);
        let steps = self.roster.steps(&readings, |character| {
            calls.object_data_is_loaded(character.object_id())
        });
        for step in steps {
            let fresh_state = players
                .iter()
                .find(|player| player.roster.slot == step.slot)
                .and_then(|player| player.fresh_state);
            match step.step {
                ExtraBodyStep::Nothing => {}
                ExtraBodyStep::RequestData(character) => {
                    self.reads.ask(&calls, character.object_id(), play.phase_id);
                }
                ExtraBodyStep::Spawn(character) => {
                    let retry_due = self
                        .failed_spawn_at_ms
                        .get(&step.slot)
                        .is_none_or(|failed| now_ms >= failed + SPAWN_RETRY_INTERVAL_MS);
                    if fresh_state.is_none() || !retry_due {
                        continue;
                    }
                    let Some(handle) =
                        make_body(&calls, step.slot, character, self.spawn_is_guarded)
                    else {
                        self.failed_spawn_at_ms.insert(step.slot, now_ms);
                        continue;
                    };
                    self.failed_spawn_at_ms.remove(&step.slot);
                    self.reads
                        .give_back(&calls, character.object_id(), play.phase_id);
                    self.roster
                        .made(step.slot, character, handle, play.phase_id);
                    self.drives
                        .entry(step.slot)
                        .or_insert_with(|| {
                            BodyDrive::new(
                                format!("{EXTRA_BODY_LOG_OWNER_PREFIX}{}", step.slot),
                                PartnerDrive::State,
                            )
                        })
                        .restart(PartnerDrive::State);
                }
                ExtraBodyStep::Drive(handle) => {
                    self.drive(step.slot, handle, now_ms, play.phase_id, fresh_state);
                }
                ExtraBodyStep::Remove(handle) => {
                    remove_body(&calls, step.slot, handle);
                    self.roster.gone(step.slot);
                }
                ExtraBodyStep::Forget(handle) => {
                    release_body(handle);
                    self.roster.gone(step.slot);
                    log_line(&format!(
                        "{EXTRA_BODY_LOG_OWNER_PREFIX}{}: the scene changed, forgot handle {handle:#X} (made again in the next scene)",
                        step.slot
                    ));
                }
            }
        }
    }

    fn drive(
        &mut self,
        slot: u8,
        handle: u32,
        now_ms: u64,
        local_phase_id: i32,
        fresh_state: Option<PlayerState>,
    ) {
        let calls = self.calls;
        let body = calls.player_body_of_handle(handle);
        if body == 0 {
            release_body(handle);
            self.roster.gone(slot);
            log_line(&format!(
                "{EXTRA_BODY_LOG_OWNER_PREFIX}{slot}: handle {handle:#X} is dead, it is made again"
            ));
            return;
        }
        let Some(drive) = self.drives.get_mut(&slot) else {
            return;
        };
        if !drive.hold_control_fields(&calls, body) || !drive.body_is_updating(&calls, body) {
            return;
        }
        let Some(remote) = fresh_state else {
            return;
        };
        let same_place = remote.phase_id == local_phase_id
            && partner_positions_apply(
                crate::travel_driver::jump_is_pending(),
                remote.jump_pending,
            );
        if same_place {
            drive.apply_remote(&calls, body, now_ms, &remote, false);
        }
    }
}

pub fn install(calls: GameCalls, spawn_is_guarded: bool) {
    let Ok(mut guard) = EXTRA_BODIES.lock() else {
        return;
    };
    *guard = Some(ExtraBodies {
        calls,
        spawn_is_guarded,
        started: Instant::now(),
        roster: ExtraBodyRoster::default(),
        drives: BTreeMap::new(),
        failed_spawn_at_ms: BTreeMap::new(),
        reads: DataRequests::default(),
    });
}

/// Makes, drives and removes the bodies of players beyond the partner. In `Campaign` it does
/// nothing at all.
pub fn tick() {
    let Some(net) = handle() else {
        return;
    };
    let mode = session_mode_of(net);
    if mode == SessionMode::Campaign {
        return;
    }
    let Ok(mut guard) = EXTRA_BODIES.try_lock() else {
        return;
    };
    let Some(bodies) = guard.as_mut() else {
        return;
    };
    let players = other_players_of(net);
    bodies.tick(mode, &players);
}
