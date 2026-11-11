use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use automata_game::address::{BLOWN_DOWN_STATE_HANDLER, STATE_DISPATCH_TABLE};
use automata_game::defeat::is_hero_defeat_state;
use automata_game::downed::{
    BLOWN_DOWN_STATE_HANDLER_START, DOWNED_HEALTH, DOWNED_LINES_LOGGED_AT_MOST, DOWNED_POSE_STATE,
    DownedEffect, DownedPhase, DownedRules, DownedView, FALL_BACK_RETRY_INTERVAL_MS,
    FALL_BACK_TRIES_AT_MOST, FallBackCause, HERO_DEFEAT_START_STATE, PoseHandlerReading,
    STATE_CASE_BYTES_READ, STATE_DISPATCH_TABLE_ENTRY_BYTES, StepInput, distance_between,
    downed_pose_start, effect_log_line, health_after_getting_up, hero_is_defeated_by_the_game,
    pose_handler_is_the_noted_one, pose_is_started_again,
};
use automata_game::exe::ExeVariant;
use automata_game::partner::{
    BODY_BASE_HEALTH_OFFSET, BODY_HEALTH_OFFSET, BODY_STATE_OFFSET, RemoteStateIds,
};
use automata_proto::PlayerState;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::{
    guard_the_hero, keep_bodies_in_the_downed_pose, take_hit_that_left_the_hero_down,
};
use crate::hud::post_downed_effect_line;
use crate::partner_driver::driver_state::PartnerDriver;

static GIVE_UP_ASKED: AtomicBool = AtomicBool::new(false);
static VIEW: Mutex<DownedView> = Mutex::new(DownedView {
    local: DownedPhase::Up,
    partner: None,
    bleed_out_progress: 0.0,
    being_revived_progress: 0.0,
    reviving_progress: 0.0,
});

#[derive(Debug, Clone, Copy)]
pub(super) struct FallBackTry {
    last_ms: u64,
    tries: u32,
}

#[derive(Debug, Default)]
pub(super) struct DownedDriver {
    pub on: bool,
    pub rules: DownedRules,
    last_step_ms: Option<u64>,
    last_hero_pose_start_ms: Option<u64>,
    last_held_body_pose_start_ms: Option<u64>,
    fall_back: Option<FallBackTry>,
    lines_logged: u32,
    partner_body_put_down_logged: bool,
}

#[derive(Debug, Clone, Copy)]
struct HeroReading {
    health: i32,
    max_health: i32,
    state: i32,
    position: Option<[f32; 3]>,
}

/// What the other player's part of the downed rules needs of the link this tick.
#[derive(Debug, Clone, Copy)]
pub(super) struct DownedLink {
    /// The other player's newest state while that player is in play with a hero.
    pub partner: Option<PlayerState>,
    pub partner_position_is_fresh: bool,
    pub defeat_flow_runs: bool,
}

impl DownedDriver {
    pub fn new(on: bool) -> Self {
        Self {
            on,
            ..Self::default()
        }
    }
}

impl PartnerDriver {
    pub(super) fn step_downed(&mut self, now_ms: u64, hero_body: Option<usize>, link: DownedLink) {
        let dt_seconds = self
            .downed
            .last_step_ms
            .replace(now_ms)
            .map_or(0.0, |last| now_ms.saturating_sub(last) as f32 / 1000.0);
        if GIVE_UP_ASKED.swap(false, Ordering::Relaxed) {
            self.downed.rules.ask_to_give_up();
        }
        let hit_left_the_hero_down = take_hit_that_left_the_hero_down();
        if let Some(hero) = hero_body
            && let Some(reading) = self.read_hero_for_the_downed_rules(hero)
        {
            let partner_distance = link
                .partner
                .filter(|partner| {
                    link.partner_position_is_fresh && partner.phase_id == self.local_phase_id
                })
                .zip(reading.position)
                .and_then(|(partner, position)| distance_between(position, partner.position));
            let effects = self.downed.rules.step(StepInput {
                dt_seconds,
                enabled: self.downed.on,
                partner: link.partner.map(|partner| partner.downed),
                partner_distance,
                local_health: reading.health,
                lethal_hit_seen: hit_left_the_hero_down,
                defeated_by_the_game: hero_is_defeated_by_the_game(
                    self.downed.rules.local(),
                    reading.health,
                    is_hero_defeat_state(reading.state),
                    link.defeat_flow_runs,
                ),
            });
            for effect in effects {
                self.apply_downed_effect(now_ms, hero, reading, effect);
            }
            self.keep_the_hero_in_the_pose(now_ms, hero, reading);
            self.see_the_fall_back_through(now_ms, hero, reading, link.defeat_flow_runs);
        }
        let local = self.downed.rules.local();
        let partner = self.downed.rules.partner();
        let partner_is_down = partner == Some(DownedPhase::Downed);
        guard_the_hero(
            hero_body.filter(|_| self.downed.on),
            self.downed.rules.hero_guard(),
        );
        keep_bodies_in_the_downed_pose(local == DownedPhase::Downed, partner_is_down);
        if partner_is_down {
            self.put_the_partner_body_down(now_ms);
        }
        *VIEW.lock().unwrap_or_else(PoisonError::into_inner) = DownedView {
            local,
            partner,
            bleed_out_progress: self.downed.rules.bleed_out_progress(),
            being_revived_progress: self.downed.rules.being_revived_progress(),
            reviving_progress: self.downed.rules.reviving_progress(),
        };
    }

    fn read_hero_for_the_downed_rules(&self, hero: usize) -> Option<HeroReading> {
        let calls = self.calls;
        let int = |offset| calls.read_i32_at(calls.body_field(hero, offset)).ok();
        Some(HeroReading {
            health: int(BODY_HEALTH_OFFSET)?,
            max_health: calls.read_max_health(hero)?,
            state: int(BODY_STATE_OFFSET)?,
            position: calls
                .read_body_position(hero)
                .ok()
                .map(|position| [position[0], position[1], position[2]]),
        })
    }

    fn apply_downed_effect(
        &mut self,
        now_ms: u64,
        hero: usize,
        reading: HeroReading,
        effect: DownedEffect,
    ) {
        self.log_downed(&effect_log_line(effect));
        post_downed_effect_line(effect);
        match effect {
            DownedEffect::WentDown => {
                self.keep_the_base_health_above_zero(hero);
                let started = self.start_the_pose(hero);
                self.downed.last_hero_pose_start_ms = Some(now_ms);
                self.log_downed(&format!(
                    "downed: pose state started on the hero: {started} (it was in state {:#X} with health {} of {}; that the end of the pose state's landing motion is a lying pose is a guess until seen)",
                    reading.state, reading.health, reading.max_health
                ));
            }
            DownedEffect::GotUp(_) => {
                let health = health_after_getting_up(reading.health, reading.max_health);
                let calls = self.calls;
                let written = health == reading.health
                    || calls
                        .write_i32_at(calls.body_field(hero, BODY_HEALTH_OFFSET), health)
                        .is_ok();
                self.downed.last_hero_pose_start_ms = None;
                self.log_downed(&format!(
                    "downed: the hero gets up with health {health} of {} (it had {}; written {written}); starts are let through again, so the pose state's own handler gets the hero up (state now {:#X})",
                    reading.max_health, reading.health, reading.state
                ));
            }
            DownedEffect::FellBack(cause) => {
                self.downed.last_hero_pose_start_ms = None;
                // A defeat the game began itself is only watched; the tries follow if it stalls.
                let by_the_mod = cause != FallBackCause::DefeatedByTheGame;
                self.downed.fall_back = Some(FallBackTry {
                    last_ms: now_ms,
                    tries: u32::from(by_the_mod),
                });
                if by_the_mod {
                    self.let_the_game_defeat_the_hero(hero, reading.state == DOWNED_POSE_STATE);
                }
            }
            _ => {}
        }
    }

    fn keep_the_hero_in_the_pose(&mut self, now_ms: u64, hero: usize, reading: HeroReading) {
        if self.downed.rules.local() != DownedPhase::Downed
            || !pose_is_started_again(reading.state, now_ms, self.downed.last_hero_pose_start_ms)
        {
            return;
        }
        let started = self.start_the_pose(hero);
        self.downed.last_hero_pose_start_ms = Some(now_ms);
        self.log_downed(&format!(
            "downed: the downed hero was in state {:#X}, not the pose state; pose state started again: {started}",
            reading.state
        ));
    }

    fn see_the_fall_back_through(
        &mut self,
        now_ms: u64,
        hero: usize,
        reading: HeroReading,
        defeat_flow_runs: bool,
    ) {
        let Some(tried) = self.downed.fall_back else {
            return;
        };
        let the_games_defeat_has_the_hero = is_hero_defeat_state(reading.state) || defeat_flow_runs;
        if the_games_defeat_has_the_hero || self.downed.rules.local() != DownedPhase::Defeated {
            self.downed.fall_back = None;
            if the_games_defeat_has_the_hero {
                self.log_downed(&format!(
                    "downed: the game's own defeat has the hero now (state {:#X}, defeat flow running: {defeat_flow_runs})",
                    reading.state
                ));
            }
            return;
        }
        if now_ms.saturating_sub(tried.last_ms) < FALL_BACK_RETRY_INTERVAL_MS {
            return;
        }
        if tried.tries >= FALL_BACK_TRIES_AT_MOST {
            self.downed.fall_back = None;
            self.log_downed(&format!(
                "downed: the game's own defeat did not take the hero after {} tries (state {:#X}, health {}); nothing more is tried",
                tried.tries, reading.state, reading.health
            ));
            return;
        }
        self.downed.fall_back = Some(FallBackTry {
            last_ms: now_ms,
            tries: tried.tries + 1,
        });
        self.log_downed(&format!(
            "downed: the game's own defeat has not taken the hero yet (state {:#X}, health {}), try {}",
            reading.state,
            reading.health,
            tried.tries + 1
        ));
        self.let_the_game_defeat_the_hero(hero, false);
    }

    fn let_the_game_defeat_the_hero(&mut self, hero: usize, pose_handler_ends_it: bool) {
        let calls = self.calls;
        let mut written = true;
        for offset in [BODY_HEALTH_OFFSET, BODY_BASE_HEALTH_OFFSET] {
            written &= calls
                .write_i32_at(calls.body_field(hero, offset), 0)
                .is_ok();
        }
        // The pose state's handler itself goes on to the end of the defeat once the base health
        // is 0; outside it the defeat state is started as the game's hit code starts it.
        let started = !pose_handler_ends_it && {
            keep_bodies_in_the_downed_pose(false, false);
            #[expect(
                unsafe_code,
                reason = "starts the hero's defeat state through its vtable"
            )]
            // SAFETY: the hero was resolved by the game's own getter this tick; the state and its
            // zero arguments are the ones the game's hit code passes for a defeat.
            unsafe {
                calls.start_state(
                    hero,
                    RemoteStateIds {
                        state: HERO_DEFEAT_START_STATE,
                        variant: 0,
                        b: 0,
                        c: 0,
                    },
                )
            }
        };
        self.log_downed(&format!(
            "downed: the hero's health and base health set to 0 (written {written}); {}",
            if pose_handler_ends_it {
                "the pose state's handler goes on to the end of the defeat by itself (a guess until run)".to_owned()
            } else {
                format!(
                    "defeat state {HERO_DEFEAT_START_STATE:#X} started on the hero: {started} (that a start from outside the game's hit code runs the same defeat is a guess until run)"
                )
            }
        ));
    }

    fn keep_the_base_health_above_zero(&mut self, hero: usize) {
        let calls = self.calls;
        let address = calls.body_field(hero, BODY_BASE_HEALTH_OFFSET);
        if calls
            .read_i32_at(address)
            .is_ok_and(|base| base < DOWNED_HEALTH)
        {
            let written = calls.write_i32_at(address, DOWNED_HEALTH).is_ok();
            self.log_downed(&format!(
                "downed: the hero's base health was below {DOWNED_HEALTH}, where the pose state's handler would end in the defeat; set to {DOWNED_HEALTH} (written {written})"
            ));
        }
    }

    fn put_the_partner_body_down(&mut self, now_ms: u64) {
        let calls = self.calls;
        let Some(body) = self
            .machine
            .held_body_handle()
            .map(|handle| calls.player_body_of_handle(handle))
            .filter(|body| *body != 0)
        else {
            return;
        };
        let Ok(state) = calls.read_i32_at(calls.body_field(body, BODY_STATE_OFFSET)) else {
            return;
        };
        if !pose_is_started_again(state, now_ms, self.downed.last_held_body_pose_start_ms) {
            return;
        }
        let started = self.start_the_pose(body);
        self.downed.last_held_body_pose_start_ms = Some(now_ms);
        if !self.downed.partner_body_put_down_logged {
            self.downed.partner_body_put_down_logged = true;
            self.log_downed(&format!(
                "downed: the other player is down, pose state started on its body here: {started} (the body was in state {state:#X}; started again whenever it is found outside the pose state; logged once)"
            ));
        }
    }

    fn start_the_pose(&self, body: usize) -> bool {
        #[expect(
            unsafe_code,
            reason = "starts the pose state through the body's vtable"
        )]
        // SAFETY: the body is the hero from the game's own getter or the held body resolved from
        // its live handle this tick, and the pose state's handler was checked at install.
        unsafe {
            self.calls.start_state(body, downed_pose_start())
        }
    }

    fn log_downed(&mut self, line: &str) {
        if self.downed.lines_logged < DOWNED_LINES_LOGGED_AT_MOST {
            self.downed.lines_logged += 1;
            log_line(line);
        }
    }
}

/// True when state 0x2D of this exe is the blown-down handler the downed pose was read from.
pub(super) fn pose_handler_is_trusted(calls: GameCalls, variant: ExeVariant) -> bool {
    let table = calls.address_value(STATE_DISPATCH_TABLE);
    let image_base = table - u64::from(STATE_DISPATCH_TABLE.rva(variant));
    let entry = table + u64::from(DOWNED_POSE_STATE as u32 * STATE_DISPATCH_TABLE_ENTRY_BYTES);
    let state_case_rva = calls.read_u32_at(entry).ok();
    let reading = PoseHandlerReading {
        state_case_rva,
        state_case_bytes: state_case_rva
            .and_then(|case| {
                calls
                    .read_bytes_at(image_base + u64::from(case), STATE_CASE_BYTES_READ)
                    .ok()
            })
            .unwrap_or_default(),
        handler_bytes: calls
            .read_bytes(
                BLOWN_DOWN_STATE_HANDLER,
                BLOWN_DOWN_STATE_HANDLER_START.len(),
            )
            .unwrap_or_default(),
    };
    pose_handler_is_the_noted_one(&reading, BLOWN_DOWN_STATE_HANDLER.rva(variant))
}

/// What the overlay shows of going down and reviving.
pub fn downed_view() -> DownedView {
    *VIEW.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn hero_is_down() -> bool {
    downed_view().local == DownedPhase::Downed
}

/// The downed player gives up waiting: the next tick lets the game's own defeat run.
pub fn ask_to_give_up() {
    GIVE_UP_ASKED.store(true, Ordering::Relaxed);
}
