use automata_game::downed::HeroGuard;
use automata_game::guest_watches::{
    HeroInputSources, StateDifference, WATCHING_LINE_INTERVAL_MS, WatchFeedReadings,
    WatchFeedRefusal, WatchingReading, hero_fields_given_back, hero_fields_while_watched,
    hero_teleported_line, hero_was_held_line, watch_feed_refusal, watch_refusal_line,
    watching_line,
};
use automata_game::minute_line_budget::MinuteLineBudget;
use automata_game::partner::{
    BODY_CONTROLLER_OFFSET, BODY_FACING_OFFSET, BODY_HEALTH_OFFSET, BODY_POSITION_OFFSET,
    BODY_ROTATION_EULER_OFFSET, BODY_STATE_OFFSET, PositionCorrection, PositionCorrector,
    facing_needs_write,
};
use automata_game::partner_flight::flight_report_of;
use automata_proto::PlayerState;
use nc_hook::diagnostics::log_line;
use nc_hook::net::NetThread;

use crate::budgeted_log::log_owned_within;
use crate::game_calls::Vec4;
use crate::game_hooks::{
    clear_flight_stick_feed, guard_the_hero, is_held_body, release_body, set_flight_stick_feed,
    set_input_feed,
};
use crate::net::AutomataNet;
use crate::partner_driver::driver_state::PartnerDriver;
use crate::partner_driver::{WATCH_REFUSAL_LINES_LOGGED_PER_MINUTE, WATCH_REFUSAL_LINES_TOPIC};

#[derive(Debug, Clone, Copy)]
struct WatchedHero {
    handle: u32,
    kept: Option<HeroInputSources>,
}

#[derive(Debug, Default)]
struct WatchDriveLogged {
    sources_cut: bool,
    flight_pad_opened: bool,
    teleported: bool,
    rider_left_in_place: bool,
}

/// The guest's own hero driven as the host's body while the guest watches.
pub(super) struct WatchDrive {
    hero: Option<WatchedHero>,
    corrector: PositionCorrector,
    state_difference: StateDifference,
    last_note: Option<String>,
    refusal_lines: MinuteLineBudget,
    last_line_ms: Option<u64>,
    logged: WatchDriveLogged,
}

/// What the watch drive needs of the link this tick.
#[derive(Debug, Clone, Copy)]
pub(super) struct WatchLink {
    pub fresh_remote: Option<PlayerState>,
    pub newest_remote: Option<PlayerState>,
    pub partner_slot: Option<u8>,
}

/// The local hero this tick: its body and its handle.
#[derive(Debug, Clone, Copy)]
pub(super) struct WatchedBody {
    pub body: usize,
    pub handle: u32,
}

impl WatchDrive {
    pub(super) fn new() -> Self {
        Self {
            hero: None,
            corrector: PositionCorrector::default(),
            state_difference: StateDifference::default(),
            last_note: None,
            refusal_lines: MinuteLineBudget::new(WATCH_REFUSAL_LINES_LOGGED_PER_MINUTE),
            last_line_ms: None,
            logged: WatchDriveLogged::default(),
        }
    }
}

impl PartnerDriver {
    /// One tick of the watched hero; true when its controller was fed.
    pub(super) fn drive_the_watched_hero(
        &mut self,
        net: &NetThread<AutomataNet>,
        now_ms: u64,
        link: WatchLink,
        hero: Option<WatchedBody>,
        local_free: bool,
    ) -> bool {
        let Some(hero) = hero else {
            self.note_the_watch_refusal(None);
            return false;
        };
        self.take_the_hero_out_of_the_guards(hero.handle);
        self.keep_the_heros_input_sources(hero);
        let host = link.fresh_remote;
        let refusal = watch_feed_refusal(WatchFeedReadings {
            stick_is_hooked: self.feed.stick,
            host_state_is_fresh: host.is_some(),
            local_phase_id: self.local_phase_id,
            host_phase_id: link.newest_remote.map(|remote| remote.phase_id),
            a_jump_is_pending: crate::travel_driver::jump_is_pending()
                || link.newest_remote.is_some_and(|remote| remote.jump_pending),
            local_free,
            host_free: host.is_some_and(|remote| remote.free),
        });
        let flight_stick = host
            .and_then(|remote| remote.flight.stick)
            .filter(|_| refusal.is_none());
        self.cut_the_heros_input_sources(hero.body, flight_stick.is_some());
        guard_the_hero(Some(hero.body), HeroGuard::Shield);
        self.note_the_watch_refusal(Some(refusal));
        let fed = self.feed.stick;
        if fed {
            let feed = link.partner_slot.map_or_else(Default::default, |slot| {
                net.update(|shared| shared.extras.remote_inputs.take_feed(slot))
            });
            let feed = if refusal.is_none() {
                feed
            } else {
                feed.stopped()
            };
            let feed = if self.feed.buttons {
                feed
            } else {
                feed.without_buttons()
            };
            set_input_feed(hero.body + BODY_CONTROLLER_OFFSET as usize, feed);
            match flight_stick {
                Some(stick) => set_flight_stick_feed(stick),
                None => clear_flight_stick_feed(),
            }
        }
        let rides = self
            .calls
            .read_flight_fields(hero.body)
            .is_some_and(|fields| flight_report_of(&fields).is_in_a_unit());
        if let Some(remote) = host {
            self.show_the_owners_health(hero.body, &remote);
            if refusal.is_none() {
                self.place_the_watched_hero(hero.body, now_ms, &remote, rides);
            }
            let hero_state = self
                .calls
                .read_i32_at(self.calls.body_field(hero.body, BODY_STATE_OFFSET));
            if let Ok(state) = hero_state
                && let Some(line) = self
                    .watch
                    .state_difference
                    .observe(now_ms, state, remote.state)
            {
                log_line(&line);
            }
        }
        self.log_the_watching_line(now_ms, hero.body, link.newest_remote, rides, refusal);
        fed
    }

    /// Gives the hero its input sources back once watching is over; nothing while it never began.
    pub(super) fn give_the_watched_hero_back(&mut self, hero: Option<WatchedBody>) {
        let Some(watched) = self.watch.hero.take() else {
            return;
        };
        self.watch.corrector = PositionCorrector::default();
        self.watch.state_difference.forget();
        self.watch.last_note = None;
        let body = match hero {
            Some(hero) if hero.handle == watched.handle => hero.body,
            _ => self.calls.player_body_of_handle(watched.handle),
        };
        if body == 0 {
            log_line(
                "watching: the watched hero is gone (a load made a new one), nothing to give back on it",
            );
            return;
        }
        self.give_the_input_sources_back(body, watched);
    }

    fn give_the_input_sources_back(&mut self, body: usize, watched: WatchedHero) {
        let calls = self.calls;
        let Ok(current) = calls.read_body_fields(body) else {
            log_line(
                "watching: cannot read the hero's fields, its input sources are not given back",
            );
            return;
        };
        let given_back = hero_fields_given_back(current, watched.kept);
        let written = calls.write_body_fields(body, &current, &given_back).is_ok();
        log_line(&format!(
            "watching: gave the hero (handle {:#X}) its input sources back: pad {}, mouse {}, keyboard {} ({}; written {written})",
            watched.handle,
            given_back.pad_enable,
            given_back.mouse_enable,
            given_back.keyboard_enable,
            if watched.kept.is_some() {
                "as they were before watching"
            } else {
                "all on, none was on when first read"
            }
        ));
    }

    fn keep_the_heros_input_sources(&mut self, hero: WatchedBody) {
        if self
            .watch
            .hero
            .is_some_and(|watched| watched.handle == hero.handle)
        {
            return;
        }
        if let Some(old) = self.watch.hero.take() {
            let old_body = self.calls.player_body_of_handle(old.handle);
            if old_body != 0 {
                self.give_the_input_sources_back(old_body, old);
            }
        }
        let kept = self
            .calls
            .read_body_fields(hero.body)
            .ok()
            .and_then(|fields| HeroInputSources::of(fields).worth_keeping());
        log_line(&format!(
            "watching: a new hero to watch (handle {:#X}); its input sources {kept:?} are kept to be given back",
            hero.handle
        ));
        self.watch.hero = Some(WatchedHero {
            handle: hero.handle,
            kept,
        });
        self.watch.corrector = PositionCorrector::default();
    }

    fn cut_the_heros_input_sources(&mut self, body: usize, flight_stick_is_fed: bool) {
        let calls = self.calls;
        let Ok(current) = calls.read_body_fields(body) else {
            return;
        };
        let wanted = hero_fields_while_watched(current, flight_stick_is_fed);
        let written = calls.write_body_fields(body, &current, &wanted).is_ok();
        if written && !self.watch.logged.sources_cut {
            self.watch.logged.sources_cut = true;
            log_line(
                "watching: the hero's pad, mouse and keyboard sources are held off every tick; its player flag, flags, input ignore timer, run speed type and dismissed flag are left as the game has them",
            );
        }
        if written && flight_stick_is_fed && !self.watch.logged.flight_pad_opened {
            self.watch.logged.flight_pad_opened = true;
            log_line(
                "watching: the host flies, so the hero's pad source is open for the fed flight stick (the fed buttons and stick overwrite the pad's after each update; logged once)",
            );
        }
    }

    fn take_the_hero_out_of_the_guards(&mut self, handle: u32) {
        if !is_held_body(handle) {
            return;
        }
        release_body(handle);
        if self.guarded_slot_body_handle == Some(handle) {
            self.guarded_slot_body_handle = None;
        }
        log_line(&hero_was_held_line(handle));
    }

    fn place_the_watched_hero(
        &mut self,
        body: usize,
        now_ms: u64,
        remote: &PlayerState,
        rides: bool,
    ) {
        let calls = self.calls;
        if rides {
            if !self.watch.logged.rider_left_in_place {
                self.watch.logged.rider_left_in_place = true;
                log_line(
                    "watching: the hero rides a flight unit; its place is not corrected while it rides, only the stick is fed (logged once)",
                );
            }
            return;
        }
        let Ok(local) = calls.read_body_position(body) else {
            return;
        };
        match self
            .watch
            .corrector
            .correct(now_ms, [local[0], local[1], local[2]], remote.position)
        {
            PositionCorrection::Leave => {}
            PositionCorrection::Nudge(position) => {
                let _ =
                    calls.write_f32s_at(calls.body_field(body, BODY_POSITION_OFFSET), &position);
            }
            PositionCorrection::Teleport => {
                self.teleport_the_watched_hero(body, local[3], remote);
                return;
            }
        }
        let facing_address = calls.body_field(body, BODY_FACING_OFFSET);
        if calls
            .read_f32_at(facing_address)
            .is_ok_and(|facing| facing_needs_write(facing, remote.body_facing))
        {
            let _ = calls.write_f32_at(facing_address, remote.body_facing);
        }
    }

    #[expect(unsafe_code, reason = "calls the game's teleport on the local hero")]
    fn teleport_the_watched_hero(&mut self, body: usize, position_w: f32, remote: &PlayerState) {
        let calls = self.calls;
        let Ok(rotation) =
            calls.read_f32s_at::<4>(calls.body_field(body, BODY_ROTATION_EULER_OFFSET))
        else {
            return;
        };
        let position = Vec4([
            remote.position[0],
            remote.position[1],
            remote.position[2],
            position_w,
        ]);
        // Zero x and z keep the game on its branch that stores the vector as the body's rotation.
        let rotation = Vec4([0.0, remote.body_facing, 0.0, rotation[3]]);
        // SAFETY: the hero came from the game's own getter this tick on the game thread. Its
        // player flag is left set, so the game runs the teleport's own part for the hero.
        unsafe { calls.teleport_reset_hap(body, &position, &rotation) };
        self.watch.state_difference.forget();
        if !self.watch.logged.teleported {
            self.watch.logged.teleported = true;
            log_line(&hero_teleported_line());
        }
    }

    fn note_the_watch_refusal(&mut self, refusal: Option<Option<WatchFeedRefusal>>) {
        // None: no hero yet; Some(None): fed; Some(Some(_)): held still for that reason.
        let line = match refusal {
            None => "watching: no hero in this game yet, nothing is driven".to_owned(),
            Some(None) => {
                "watching: the hero is fed the host's input and follows its place".to_owned()
            }
            Some(Some(refusal)) => watch_refusal_line(refusal),
        };
        if self.watch.last_note.as_deref() == Some(line.as_str()) {
            return;
        }
        self.watch.last_note = Some(line.clone());
        log_owned_within(
            &mut self.watch.refusal_lines,
            WATCH_REFUSAL_LINES_TOPIC,
            line,
        );
    }

    fn log_the_watching_line(
        &mut self,
        now_ms: u64,
        body: usize,
        host: Option<PlayerState>,
        rides: bool,
        refusal: Option<WatchFeedRefusal>,
    ) {
        let due = self
            .watch
            .last_line_ms
            .is_none_or(|last| now_ms.saturating_sub(last) >= WATCHING_LINE_INTERVAL_MS);
        if !due {
            return;
        }
        self.watch.last_line_ms = Some(now_ms);
        let calls = self.calls;
        let int = |offset| calls.read_i32_at(calls.body_field(body, offset)).ok();
        log_line(&watching_line(&WatchingReading {
            hero_position: calls
                .read_body_position(body)
                .ok()
                .map(|position| [position[0], position[1], position[2]]),
            host_position: host.map(|remote| remote.position),
            hero_state: int(BODY_STATE_OFFSET),
            host_state: host.map(|remote| remote.state),
            hero_health: int(BODY_HEALTH_OFFSET),
            host_health: host.map(|remote| (remote.health, remote.max_health)),
            hero_rides: rides,
            refusal,
        }));
    }
}
