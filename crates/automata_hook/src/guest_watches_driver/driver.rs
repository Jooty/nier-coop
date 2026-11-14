use std::panic;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

use automata_game::address::HERO_MODEL_ID_VALUE;
use automata_game::character::Character;
use automata_game::defeat::{DefeatFlow, is_hero_defeat_state};
use automata_game::guest_watches::{
    FullLoadVerdict, GuestWatch, WatchReadings, WatchRole, WatchedStretch, guest_watch_applies,
    hand_back_line, hero_written_for_a_start, host_defeat_seen_while_watching_line,
    host_defeated_while_watched_line, host_is_watched, host_watched_line, own_hits_kept_back_line,
    own_pickups_kept_back_line, start_hero_line, watching_setting_line,
};
use automata_game::load_gate::StoryPlace;
use automata_game::partner::BODY_MODEL_ID_OFFSET;
use automata_game::save_redirect::SaveRole;
use automata_game::travel::JumpRole;
use automata_proto::PlayerState;
use nt_hook::build_gate::VerifiedBuild;
use nt_hook::diagnostics::log_line;
use nt_hook::net::net_clock_ms;

use crate::game_calls::GameCalls;
use crate::net::{
    SessionModeExt, current_session_mode, handle, local_story_place, set_local_watching,
};
use crate::start_driver::link_reading;
use crate::start_setting::GameSettings;

static CONTEXT: Mutex<Option<WatchContext>> = Mutex::new(None);
static WATCH_ROLE_NOW: AtomicU8 = AtomicU8::new(WatchRole::Neither.as_number());
static OWN_HITS_KEPT_BACK: AtomicUsize = AtomicUsize::new(0);
static OWN_HITS_KEPT_BACK_LOGGED: AtomicBool = AtomicBool::new(false);
static OWN_PICKUPS_KEPT_BACK: AtomicUsize = AtomicUsize::new(0);
static OWN_PICKUPS_KEPT_BACK_LOGGED: AtomicBool = AtomicBool::new(false);
static BUSY_AT_A_FULL_REQUEST_LOGGED: AtomicBool = AtomicBool::new(false);

struct WatchContext {
    calls: GameCalls,
    own: Character,
    host_setting: Character,
    is_the_guest_in_a_session: bool,
    machine: GuestWatch,
    host_watched_logged: Option<bool>,
    host_defeat_flow_seen: bool,
    host_defeat_seen_by_the_guest: bool,
}

struct HostView {
    character: Option<Character>,
    newest: Option<PlayerState>,
}

pub fn install(build: VerifiedBuild, settings: &GameSettings) {
    let stretch = WatchedStretch::new(settings.watching_ends_at.as_deref());
    log_line(&watching_setting_line(
        settings.guest_watches_the_opening,
        &stretch,
    ));
    if !settings.guest_watches_the_opening {
        return;
    }
    let Some(variant) = automata_game::build::variant_of_image_size(build.size_of_image()) else {
        return;
    };
    if let Ok(mut guard) = CONTEXT.lock() {
        *guard = Some(WatchContext {
            calls: GameCalls::new(build, variant),
            own: settings.character,
            host_setting: settings.partner,
            is_the_guest_in_a_session: settings.role == SaveRole::Guest
                && settings.link_port.is_some(),
            machine: GuestWatch::new(stretch),
            host_watched_logged: None,
            host_defeat_flow_seen: false,
            host_defeat_seen_by_the_guest: false,
        });
    }
}

pub fn tick() {
    let _ = panic::catch_unwind(|| {
        let Ok(mut guard) = CONTEXT.try_lock() else {
            return;
        };
        if let Some(context) = guard.as_mut() {
            context.step();
        }
    });
}

/// What watching makes of this copy, as of the last tick; game thread only.
pub fn watch_role_now() -> WatchRole {
    WatchRole::from_number(WATCH_ROLE_NOW.load(Ordering::Relaxed))
}

/// The hero a start into `target` writes instead of its own choice; none when this copy will
/// not watch there. Game thread only.
pub fn character_for_a_start(target: &StoryPlace) -> Option<Character> {
    let guard = CONTEXT.try_lock().ok()?;
    let context = guard.as_ref()?;
    let host = context
        .host_view()
        .character
        .unwrap_or(context.host_setting);
    let in_this_session =
        context.is_the_guest_in_a_session && current_session_mode().story_rules_apply();
    hero_written_for_a_start(in_this_session, context.machine.stretch(), target, host)
}

/// Logs what a start wrote for the hero of a watched place.
pub fn note_the_start_hero(target: &StoryPlace, host: Character, written: bool) {
    log_line(&start_hero_line(target, host, written));
}

/// Called by the scene request hook just before a full load is asked for; game thread only.
pub fn before_a_full_request(phase_id: i32, name: &str) {
    let Ok(mut guard) = CONTEXT.try_lock() else {
        if !BUSY_AT_A_FULL_REQUEST_LOGGED.swap(true, Ordering::Relaxed) {
            log_line(
                "watching: a full load was asked for while the watch state was busy; it is not seen as a hand-back (logged once)",
            );
        }
        return;
    };
    let Some(context) = guard.as_mut() else {
        return;
    };
    let target = StoryPlace::new(phase_id, name);
    if context.machine.before_a_full_load(net_clock_ms(), &target) == FullLoadVerdict::HandsBack {
        let written = context
            .calls
            .write_u32(HERO_MODEL_ID_VALUE, context.own.object_id())
            .is_ok();
        log_line(&hand_back_line(&target, context.own, written));
    }
}

/// True while this copy watches: its hero's hits on enemies are not told; they are counted.
pub fn own_hits_are_kept_back(count: usize) -> bool {
    keep_back(
        count,
        &OWN_HITS_KEPT_BACK,
        &OWN_HITS_KEPT_BACK_LOGGED,
        own_hits_kept_back_line,
    )
}

/// True while this copy watches: items given to its hero are not told; they are counted.
pub fn own_pickups_are_kept_back(count: usize) -> bool {
    keep_back(
        count,
        &OWN_PICKUPS_KEPT_BACK,
        &OWN_PICKUPS_KEPT_BACK_LOGGED,
        own_pickups_kept_back_line,
    )
}

fn keep_back(
    count: usize,
    kept: &AtomicUsize,
    logged: &AtomicBool,
    line: fn(usize) -> String,
) -> bool {
    if watch_role_now() != WatchRole::Watches {
        return false;
    }
    let total = kept.fetch_add(count, Ordering::Relaxed) + count;
    if count > 0 && !logged.swap(true, Ordering::Relaxed) {
        log_line(&line(total));
    }
    true
}

impl WatchContext {
    fn step(&mut self) {
        let link = link_reading();
        let host = self.host_view();
        let now_ms = net_clock_ms();
        let readings = WatchReadings {
            now_ms,
            applies: guest_watch_applies(true, link.role, link.partner_present),
            place: local_story_place(),
            in_play: self.calls.read_free().in_play,
            hero: self.hero_character(),
            host: host.character,
        };
        let step = self.machine.step(&readings);
        for line in &step.logs {
            log_line(line);
        }
        set_local_watching(step.watching);
        let host_watched = host_is_watched(
            true,
            link.role,
            host.newest.is_some_and(|state| state.watching),
        );
        let role = if step.watching {
            WatchRole::Watches
        } else if host_watched {
            WatchRole::IsWatched
        } else {
            WatchRole::Neither
        };
        WATCH_ROLE_NOW.store(role.as_number(), Ordering::Relaxed);
        self.note_the_host_side(link.role, host_watched);
        self.note_a_host_defeat(role, host.newest);
    }

    fn host_view(&self) -> HostView {
        handle()
            .map(|net| {
                net.update(|shared| {
                    let partner = shared.campaign_partner();
                    HostView {
                        character: partner.and_then(|player| player.watch.character),
                        newest: partner
                            .and_then(|player| player.latest.as_ref())
                            .map(|latest| latest.state),
                    }
                })
            })
            .unwrap_or(HostView {
                character: None,
                newest: None,
            })
    }

    fn hero_character(&self) -> Option<Character> {
        let calls = &self.calls;
        let body = calls.current_hero();
        (body != 0)
            .then(|| {
                calls
                    .read_u32_at(calls.body_field(body, BODY_MODEL_ID_OFFSET))
                    .ok()
            })
            .flatten()
            .and_then(Character::from_model_id)
    }

    fn note_the_host_side(&mut self, role: JumpRole, host_watched: bool) {
        if role != JumpRole::Host {
            return;
        }
        if self.host_watched_logged != Some(host_watched)
            && (host_watched || self.host_watched_logged.is_some())
        {
            log_line(&host_watched_line(host_watched));
        }
        self.host_watched_logged = Some(host_watched);
    }

    fn note_a_host_defeat(&mut self, role: WatchRole, host: Option<PlayerState>) {
        let flow_runs = role == WatchRole::IsWatched
            && self
                .calls
                .read_defeat_flow()
                .is_some_and(DefeatFlow::is_active);
        if flow_runs && !self.host_defeat_flow_seen {
            log_line(&host_defeated_while_watched_line());
        }
        self.host_defeat_flow_seen = flow_runs;
        let host_defeated = role == WatchRole::Watches
            && host.is_some_and(|state| {
                state.health <= 0 || is_hero_defeat_state(i32::from(state.state))
            });
        if host_defeated
            && !self.host_defeat_seen_by_the_guest
            && let Some(state) = host
        {
            log_line(&host_defeat_seen_while_watching_line(
                state.state,
                state.health,
            ));
        }
        self.host_defeat_seen_by_the_guest = host_defeated;
    }
}
