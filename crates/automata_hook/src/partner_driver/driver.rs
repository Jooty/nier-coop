use std::time::{Duration, Instant};

use automata_game::address::POSSESSED_HANDLE_VALUE;
use automata_game::autopilot::{Autopilot, autopilot_feed};
use automata_game::build::variant_of_image_size;
use automata_game::character::Character;
use automata_game::defeat::{DefeatFlowLog, HealthLog, HoldLog};
use automata_game::downed::{DOWNED_POSE_STATE, DownedPhase};
use automata_game::partner::BodySnapshot;
use automata_game::partner::{
    BODY_BASE_HEALTH_OFFSET, BODY_BERSERK_SWITCH_OFFSET, BODY_BREAK_FIELD_COUNT,
    BODY_BREAK_FIELDS_OFFSET, BODY_CONTROLLER_OFFSET, BODY_FACING_OFFSET, BODY_FLAGS_OFFSET,
    BODY_HEALTH_OFFSET, BODY_MODEL_ID_OFFSET, BODY_PARTNER_MARK_WORD_OFFSET,
    BODY_PARTNER_SLOT_HANDLE_OFFSET, BODY_PLAYER_CONTROLLED_OFFSET, BODY_POSITION_OFFSET,
    BODY_ROTATION_EULER_OFFSET, BODY_STATE_OFFSET, BODY_SUSPEND_FLAGS_OFFSET,
    BODY_UPDATE_GATE_OFFSET, BuddyReading, COMPUTER_CONTROLLED_FLAG_BITS, ENTITY_HANDLE_OFFSET,
    FREE_MUST_HOLD_BEFORE_IT_COUNTS_MS, FreeReadings, GiveBack, HeldFree, HelloRule, HeroIdentity,
    HeroReading, PARTNER_MARK_BIT, POSITION_ERROR_TELEPORTED, PartnerAction, PartnerDataReading,
    PartnerDrive, PartnerMachine, PartnerReadings, PositionCorrection, PositionCorrector,
    REMOTE_STATE_STALE_MS, RemoteState, SPAWN_OFFSET_FROM_HERO, SPAWNED_BODY_NAME,
    SUSPENDED_FLAG_BIT, SendPacer, StateDecision, StateRule, facing_needs_write, gives_fields_back,
    not_free_reason, partner_body_status_line, partner_is_in_play, partner_positions_apply,
    wanted_partner,
};
use automata_proto::{CharacterHello, Message, PlayerState};
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;
use nc_hook::net::{NetState, NetThread};

use crate::game_calls::{GameCalls, Vec4};
use crate::game_hooks::{
    auto_item_use_checks_skipped_on_the_partner_body, clear_autopilot_feed, clear_input_feed,
    forget_recorded_start, guard_held_body, hits_kept_off_the_partner_body, install_camera_guard,
    install_engine_partner_choice, install_focus_pass, install_input_feed,
    install_object_read_wait_guard, install_own_input_guard, install_own_item_use_hold,
    install_partner_harm_guard, install_partner_walk_guards, install_pod_aim,
    install_start_recorder, object_read_waits_refused, recorded_pod_aim, recorded_start,
    set_autopilot_feed, set_input_feed, set_partner_pod_aim, set_partner_the_engine_should_make,
    set_session_linked,
};
use crate::net::{AutomataNet, handle, start_link};
use crate::partner_driver::downed_watch::{DownedDriver, DownedLink, pose_handler_is_trusted};
use crate::partner_driver::driver_state::{
    BODY_STATUS_LINE, DRIVER, PartnerDriver, RequestedRead, StagesLogged,
};
use crate::partner_driver::{
    BODY_READING_LINE_INTERVAL_MS, BODY_READING_LINES_LOGGED_AT_MOST,
    FREE_CHANGE_LINES_LOGGED_AT_MOST,
};
use crate::start_setting::GameSettings;

const NO_SESSION_BODY_STATUS_TEXT: &str = "partner body: no session";
const HERO_HEALTH_LOG_NAME: &str = "the hero";
const OTHER_PLAYER_HEALTH_LOG_NAME: &str = "the other player";

struct LinkView {
    connected: bool,
    told: Option<Character>,
    partner_heard: bool,
    newest_state_age_ms: Option<u64>,
    newest_remote: Option<PlayerState>,
    fresh_remote: Option<PlayerState>,
}

#[derive(Clone, Copy)]
struct Hero {
    body: usize,
    reading: HeroReading,
    character: Option<Character>,
}

#[derive(Debug, Clone, Copy)]
struct FreeNow {
    both_free: bool,
    local_free_now: bool,
    other_free_now: bool,
}

impl PartnerDriver {
    fn tick(&mut self, net: &NetThread<AutomataNet>) {
        let now_ms = self.started.elapsed().as_millis() as u64;
        let link = net.update(|shared| LinkView {
            connected: shared.state == NetState::Connected,
            told: shared.extras.partner_character,
            partner_heard: shared.extras.partner_heard,
            newest_state_age_ms: shared
                .remote
                .as_ref()
                .map(|remote| remote.age().as_millis() as u64),
            newest_remote: shared.remote.as_ref().map(|remote| remote.state),
            fresh_remote: shared
                .remote
                .as_ref()
                .filter(|remote| remote.age() <= Duration::from_millis(REMOTE_STATE_STALE_MS))
                .map(|remote| remote.state),
        });
        set_session_linked(link.connected);
        let hero = self.read_hero();
        self.forget_the_starts_of_another_hero(hero);
        let free = self.calls.read_free();
        let local_free = self.note_free(&free);
        self.local_phase_id = self.calls.read_play(hero.is_some()).phase_id;
        self.tell_character(net, &link, hero);
        let wanted = wanted_partner(
            self.partner_setting,
            link.told,
            hero.and_then(|hero| hero.character),
        );
        set_partner_the_engine_should_make(
            (link.connected && link.partner_heard).then_some(wanted),
        );
        if free.in_play {
            self.publish_local_state(net, now_ms, hero, local_free);
        }
        self.run_autopilot(now_ms, hero, local_free);
        let defeat_flow_runs =
            self.watch_health_and_defeat(now_ms, hero.map(|hero| hero.body), link.connected);
        self.step_downed(
            now_ms,
            hero.map(|hero| hero.body),
            DownedLink {
                partner: link.newest_remote.filter(|remote| {
                    partner_is_in_play(link.connected, link.newest_state_age_ms)
                        && !remote.load.without_body
                }),
                partner_position_is_fresh: link.fresh_remote.is_some(),
                defeat_flow_runs,
            },
        );
        let both_free = self.note_both_held_free(now_ms, local_free, &link);
        if link.connected {
            self.note_hold(now_ms, both_free, defeat_flow_runs, link.newest_remote);
            self.note_the_other_players_health(link.newest_remote);
        }
        let other_free_now = link.newest_remote.is_some_and(|remote| remote.free);
        let fed = self.step_partner(
            net,
            now_ms,
            &link,
            hero,
            FreeNow {
                both_free,
                local_free_now: local_free,
                other_free_now,
            },
        );
        set_partner_pod_aim(
            link.fresh_remote
                .filter(|_| fed)
                .and_then(|remote| remote.pod_aim),
        );
        if !fed {
            clear_input_feed();
            net.update(|shared| shared.extras.remote_input.drop_presses());
        }
    }

    fn note_free(&mut self, free: &FreeReadings) -> bool {
        let reason = not_free_reason(free);
        self.local_not_free = reason;
        if self.free_reason_logged != Some(reason)
            && self.free_change_lines < FREE_CHANGE_LINES_LOGGED_AT_MOST
        {
            self.free_reason_logged = Some(reason);
            self.free_change_lines += 1;
            log_line(&match reason {
                None => "free: the local player is free (in play, and none of the game's input gates is set)".to_owned(),
                Some(reason) => format!("free: the local player is not free: {reason}"),
            });
            if self.free_change_lines == FREE_CHANGE_LINES_LOGGED_AT_MOST {
                log_line("free: no more lines about this are logged");
            }
        }
        reason.is_none()
    }

    fn note_both_held_free(&mut self, now_ms: u64, local_free: bool, link: &LinkView) -> bool {
        let remote_free = link.newest_remote.is_some_and(|remote| remote.free);
        let local_held = self.local_held_free.observe(now_ms, local_free);
        let remote_held = self.remote_held_free.observe(now_ms, remote_free);
        let both = local_held && remote_held;
        if self.both_held_free_logged != Some(both)
            && self.free_change_lines < FREE_CHANGE_LINES_LOGGED_AT_MOST
        {
            self.both_held_free_logged = Some(both);
            self.free_change_lines += 1;
            log_line(&if both {
                format!(
                    "free: both players have been free for {FREE_MUST_HOLD_BEFORE_IT_COUNTS_MS} ms without a break"
                )
            } else {
                format!(
                    "free: not both players count as free (local free {local_free}, held {local_held}; other player free {remote_free}, held {remote_held})"
                )
            });
        }
        both
    }

    fn run_autopilot(&self, now_ms: u64, hero: Option<Hero>, local_free: bool) {
        let feed = autopilot_feed(self.autopilot, local_free && self.feed.stick, now_ms);
        match (hero, feed) {
            (Some(hero), Some(feed)) => {
                set_autopilot_feed(hero.body + BODY_CONTROLLER_OFFSET as usize, feed);
            }
            _ => clear_autopilot_feed(),
        }
    }

    fn forget_the_starts_of_another_hero(&mut self, hero: Option<Hero>) {
        let hero_handle = hero.map(|hero| hero.reading.handle);
        if hero_handle != self.last_hero_handle {
            self.last_hero_handle = hero_handle;
            forget_recorded_start();
        }
    }

    fn read_hero(&self) -> Option<Hero> {
        let calls = &self.calls;
        let handle = calls
            .read_u32(POSSESSED_HANDLE_VALUE)
            .ok()
            .filter(|handle| *handle != 0)?;
        let body = calls.current_hero();
        if body == 0 {
            return None;
        }
        let flags = calls
            .read_u32_at(calls.body_field(body, BODY_FLAGS_OFFSET))
            .ok()?;
        let player_flag = calls
            .read_i32_at(calls.body_field(body, BODY_PLAYER_CONTROLLED_OFFSET))
            .ok()?;
        let character = calls
            .read_u32_at(calls.body_field(body, BODY_MODEL_ID_OFFSET))
            .ok()
            .and_then(Character::from_model_id);
        Some(Hero {
            body,
            character,
            reading: HeroReading {
                handle,
                player_controlled: player_flag != 0,
                computer_controlled: flags & COMPUTER_CONTROLLED_FLAG_BITS != 0,
            },
        })
    }

    fn tell_character(
        &mut self,
        net: &NetThread<AutomataNet>,
        link: &LinkView,
        hero: Option<Hero>,
    ) {
        let identity = hero.map(|hero| HeroIdentity {
            handle: hero.reading.handle,
            character: hero.character,
        });
        let to_tell = self.hello.character_to_tell(
            link.connected,
            self.local_character,
            identity,
            link.partner_heard,
        );
        if let Some(character) = to_tell {
            net.send_relayed(Message::Character(CharacterHello {
                character,
                exe: self.local_exe,
            }));
            log_line(&format!(
                "net: told the other player our character ({}, {})",
                character.launcher_id(),
                if hero.is_some_and(|hero| hero.character.is_some()) {
                    "the hero's model"
                } else {
                    "the setting, no hero model read yet"
                }
            ));
        }
    }

    fn publish_local_state(
        &mut self,
        net: &NetThread<AutomataNet>,
        now_ms: u64,
        hero: Option<Hero>,
        local_free: bool,
    ) {
        let Some(hero) = hero else {
            return;
        };
        if !net.wants_snapshots() {
            return;
        }
        let last_start = recorded_start().filter(|_| self.starts_are_recorded);
        let read = self
            .calls
            .read_pressed_buttons(hero.body)
            .map(|pressed| self.pacer.observe(now_ms, pressed))
            .and_then(|due| {
                due.map(|pressed| {
                    self.calls
                        .read_body_snapshot(hero.body, pressed, last_start)
                        .map(|snapshot| BodySnapshot {
                            free: local_free,
                            phase_id: self.local_phase_id,
                            paused: self.calls.read_game_is_paused(),
                            jump_pending: crate::travel_driver::jump_is_pending(),
                            pod_aim: recorded_pod_aim(),
                            downed: self.downed.rules.signal(self.downed.on),
                            ..snapshot
                        })
                })
                .transpose()
            });
        match read {
            Ok(None) => {}
            Ok(Some(snapshot)) => {
                net.publish(snapshot);
                if !self.logged.local_state_sent {
                    self.logged.local_state_sent = true;
                    log_line(&format!(
                        "net: first local state handed to the net worker (state {:#X}, health {})",
                        snapshot.state, snapshot.health
                    ));
                }
            }
            Err(error) => {
                if !self.logged.local_state_unreadable {
                    self.logged.local_state_unreadable = true;
                    log_line(&format!("net: cannot read the hero's state ({error})"));
                }
            }
        }
    }

    fn step_partner(
        &mut self,
        net: &NetThread<AutomataNet>,
        now_ms: u64,
        link: &LinkView,
        hero: Option<Hero>,
        free_now: FreeNow,
    ) -> bool {
        let partner_in_play = partner_is_in_play(link.connected, link.newest_state_age_ms);
        let held_handle = self.machine.held_body_handle();
        if !partner_in_play && held_handle.is_none() {
            self.publish_body_status(None);
            return false;
        }
        let calls = self.calls;
        let held_body = held_handle
            .map(|handle| calls.player_body_of_handle(handle))
            .filter(|body| *body != 0);
        let buddy_body = hero
            .map(|_| calls.current_buddy())
            .filter(|body| *body != 0);
        let other_slot_holder = match (hero, buddy_body) {
            (Some(hero), None) => self.read_other_slot_holder(hero.body),
            _ => None,
        };
        let wanted = wanted_partner(
            self.partner_setting,
            link.told,
            hero.and_then(|hero| hero.character),
        );
        let buddy = buddy_body.and_then(|body| self.read_buddy(body));
        let held_object_id = held_body.and_then(|body| self.read_model_id(body));
        let readings = PartnerReadings {
            partner_in_play,
            both_free: free_now.both_free,
            local_free_now: free_now.local_free_now,
            other_free_now: free_now.other_free_now,
            wanted,
            hero: hero.map(|hero| hero.reading),
            buddy,
            other_slot_holder,
            held_object_id,
            wanted_data_is_loaded: self.wanted_data_is_loaded(wanted, hero, buddy),
        };
        self.publish_body_status(Some(partner_body_status_line(wanted, held_object_id)));
        let step = self.machine.step(now_ms, &readings);
        log_all(step.logs);
        match step.action {
            PartnerAction::Nothing => {}
            PartnerAction::RequestObjectRead { object_id } => {
                self.request_object_read(object_id);
            }
            PartnerAction::Adopt => {
                if let Some(body) = buddy_body {
                    self.take_over(body);
                }
                if buddy.is_some_and(|buddy| buddy.object_id == wanted.object_id()) {
                    self.release_the_read_asked_for(
                        wanted.object_id(),
                        "the engine made the body, which holds its own data now",
                    );
                }
            }
            PartnerAction::DestroySlotPartner => {
                if let (Some(hero), Some(buddy)) = (hero, buddy_body) {
                    self.destroy_slot_partner(hero.body, buddy);
                }
            }
            PartnerAction::Spawn { object_id } => {
                if let Some(hero) = hero {
                    self.spawn(object_id, hero);
                }
            }
            PartnerAction::Drive => {
                if let Some(body) = held_body {
                    return self.drive(net, body, now_ms, link);
                }
            }
            PartnerAction::Hold => {
                if let Some(body) = held_body {
                    self.hold_control_fields(body);
                }
                clear_input_feed();
            }
            PartnerAction::Relink => {
                if let (Some(hero), Some(body)) = (hero, held_body) {
                    self.link_again(hero.body, body);
                }
            }
            PartnerAction::Release {
                restore_fields,
                terminate,
                made_by_mod,
                mark_dismissed,
            } => self.release(
                held_handle,
                held_body,
                restore_fields,
                terminate,
                GiveBack {
                    made_by_mod,
                    mark_dismissed,
                },
            ),
        }
        false
    }

    fn wanted_data_is_loaded(
        &mut self,
        wanted: Character,
        hero: Option<Hero>,
        buddy: Option<BuddyReading>,
    ) -> bool {
        if hero.is_none() {
            return false;
        }
        if buddy.is_some_and(|buddy| buddy.object_id == wanted.object_id()) {
            return true;
        }
        if !self.spawn_is_guarded {
            if !self.logged.spawn_not_guarded {
                self.logged.spawn_not_guarded = true;
                log_line(
                    "partner: the game's wait for an object read is not guarded, so no body is spawned; the engine's own partner is kept",
                );
            }
            return false;
        }
        self.calls.object_data_is_loaded(wanted.object_id())
    }

    fn request_object_read(&mut self, object_id: u32) -> bool {
        let request = RequestedRead {
            object_id,
            phase_id: self.local_phase_id,
        };
        if self.requested_read == Some(request) {
            log_line("partner: the read was already asked for in this place, waiting for it");
            return true;
        }
        if !self.spawn_is_guarded {
            return false;
        }
        if let Some(other) = self.requested_read
            && other.object_id != object_id
        {
            self.release_the_read_asked_for(other.object_id, "another character is wanted now");
        }
        let taken = self.calls.request_object_read(object_id);
        if taken {
            self.requested_read = Some(request);
            log_line(&format!(
                "partner: the game took the request to read object {object_id:#X}"
            ));
        } else {
            log_line(&format!(
                "partner: the game refused to read object {object_id:#X} (no such object file or no free read slot); the engine's own partner is kept"
            ));
        }
        taken
    }

    fn read_partner_data_ahead(&self) -> Option<PartnerDataReading> {
        if !self.spawn_is_guarded {
            return None;
        }
        let told = handle().and_then(|net| net.update(|shared| shared.extras.partner_character));
        let wanted = wanted_partner(
            self.partner_setting,
            told,
            self.read_hero().and_then(|hero| hero.character),
        );
        Some(PartnerDataReading {
            wanted,
            loaded: self.calls.object_data_is_loaded(wanted.object_id()),
        })
    }

    fn release_the_read_asked_for(&mut self, object_id: u32, why: &str) {
        let Some(request) = self.requested_read.take() else {
            return;
        };
        // After a scene change the read system may hold other counts; a count is left rather
        // than taken twice.
        if request.object_id == object_id && request.phase_id == self.local_phase_id {
            self.calls.release_object_read(object_id);
            log_line(&format!(
                "partner: gave back the read request of object {object_id:#X} ({why})"
            ));
        }
    }

    fn publish_body_status(&self, line: Option<String>) {
        let line = line.unwrap_or_default();
        if let Ok(mut status) = BODY_STATUS_LINE.try_lock()
            && *status != line
        {
            *status = line;
        }
    }

    fn read_other_slot_holder(&self, hero_body: usize) -> Option<u32> {
        let calls = &self.calls;
        let handle = calls
            .read_u32_at(calls.body_field(hero_body, BODY_PARTNER_SLOT_HANDLE_OFFSET))
            .ok()
            .filter(|handle| *handle != 0)?;
        // The partner getter already said this is no player body; alive means something else.
        (calls.entity_of_handle(handle) != 0).then_some(handle)
    }

    fn read_model_id(&self, body: usize) -> Option<u32> {
        self.calls
            .read_u32_at(self.calls.body_field(body, BODY_MODEL_ID_OFFSET))
            .ok()
    }

    fn read_buddy(&self, body: usize) -> Option<BuddyReading> {
        Some(BuddyReading {
            handle: self.calls.handle_of_body(body).ok()?,
            object_id: self.read_model_id(body)?,
        })
    }

    fn take_over(&mut self, body: usize) {
        let calls = &self.calls;
        match calls.read_body_fields(body) {
            Ok(fields) => {
                let partner_mark = calls
                    .read_u32_at(calls.body_field(body, BODY_PARTNER_MARK_WORD_OFFSET))
                    .map_or("unreadable".to_owned(), |word| {
                        u8::from(word & PARTNER_MARK_BIT != 0).to_string()
                    });
                let berserk_switch = calls
                    .read_i32_at(calls.body_field(body, BODY_BERSERK_SWITCH_OFFSET))
                    .map_or("unreadable".to_owned(), |switch| switch.to_string());
                log_line(&format!(
                    "partner: saved the body's fields before taking it over: flags {:#X}, player flag {}, enables {}/{}/{}, input ignore {}, run speed type {}, dismissed {}; partner mark bit {partner_mark} and the berserk switch ({berserk_switch}) are left as they are",
                    fields.flags,
                    fields.player_controlled,
                    fields.pad_enable,
                    fields.mouse_enable,
                    fields.keyboard_enable,
                    fields.input_ignore_seconds,
                    fields.run_speed_type,
                    fields.dismissed
                ));
                self.saved_fields = Some(fields);
            }
            Err(error) => {
                self.saved_fields = None;
                log_line(&format!(
                    "partner: cannot read the body's fields ({error}); nothing will be put back"
                ));
            }
        }
        guard_held_body(self.machine.held_body_handle());
        self.state_rule = StateRule::new(self.drive);
        self.corrector = PositionCorrector::default();
        self.last_remote_sequence = None;
    }

    #[expect(unsafe_code, reason = "calls the game's partner removal")]
    fn destroy_slot_partner(&mut self, hero_body: usize, buddy: usize) {
        let calls = &self.calls;
        let flag_address = calls.body_field(buddy, BODY_PLAYER_CONTROLLED_OFFSET);
        let flag_is_clear = match calls.read_i32_at(flag_address) {
            Ok(0) => true,
            Ok(_) => calls.write_i32_at(flag_address, 0).is_ok(),
            Err(_) => false,
        };
        if !flag_is_clear {
            log_line(
                "partner: cannot clear the slot partner's player flag, so it is not removed (the game's walk would never end)",
            );
            return;
        }
        // SAFETY: the hero came from the game's getter this tick and the partner's player flag
        // was just read or written as 0.
        unsafe { calls.destroy_buddy(hero_body) };
    }

    #[expect(
        unsafe_code,
        reason = "links, places or removes the entity the spawn just returned"
    )]
    fn spawn(&mut self, object_id: u32, hero: Hero) {
        let calls = self.calls;
        if !self.spawn_is_guarded || !calls.object_data_is_loaded(object_id) {
            log_all(
                self.machine
                    .spawn_failed("the data of the body is not loaded or the wait is not guarded"),
            );
            return;
        }
        let mut name = SPAWNED_BODY_NAME.as_bytes().to_vec();
        name.push(0);
        let waits_refused_before = object_read_waits_refused();
        // SAFETY: the data was just read as loaded and the wait guard is installed.
        let entity = unsafe { calls.spawn_by_name(&name, object_id) };
        let waits_refused = object_read_waits_refused().wrapping_sub(waits_refused_before);
        if waits_refused != 0 {
            log_line(&format!(
                "partner: the spawn of object {object_id:#X} wanted to wait for {waits_refused} object read(s) that were refused; something the body needs is not loaded"
            ));
        }
        self.release_the_read_asked_for(object_id, "the body holds its own data now");
        if entity == 0 {
            log_all(self.machine.spawn_failed(
                "the spawn returned 0: the entity table is full or an object it needs is not loaded",
            ));
            return;
        }
        // SAFETY: the entity was returned by the game's spawn in this call and is alive.
        let body = unsafe { calls.as_player_body(entity) };
        if body == 0 {
            // SAFETY: the entity is the one this mod spawned a moment ago.
            unsafe { calls.terminate_entity(entity) };
            log_all(
                self.machine
                    .spawn_failed("the spawned entity is not a player body, removed it"),
            );
            return;
        }
        let handle_address = entity + ENTITY_HANDLE_OFFSET as usize;
        // SAFETY: the hero came from the game's getter this tick; the handle field belongs to the
        // live entity spawned above.
        unsafe { calls.set_buddy_handle(hero.body, handle_address) };
        let handle = calls.read_u32_at(handle_address as u64).unwrap_or(0);
        if calls.current_buddy() != body || handle == 0 {
            // SAFETY: the entity is the one this mod spawned a moment ago.
            unsafe { calls.terminate_entity(entity) };
            log_all(
                self.machine
                    .spawn_failed("the hero's partner slot did not take the body, removed it"),
            );
            return;
        }
        self.place_beside_hero(body, hero.body);
        log_all(self.machine.spawned(handle, hero.reading.handle));
        self.take_over(body);
    }

    fn place_beside_hero(&self, body: usize, hero_body: usize) {
        let calls = &self.calls;
        let position = calls.read_body_position(hero_body);
        let rotation =
            calls.read_f32s_at::<4>(calls.body_field(hero_body, BODY_ROTATION_EULER_OFFSET));
        let (Ok(position), Ok(rotation)) = (position, rotation) else {
            log_line("partner: cannot read the hero's position, the body stays where it spawned");
            return;
        };
        let beside = Vec4([
            position[0] + SPAWN_OFFSET_FROM_HERO[0],
            position[1] + SPAWN_OFFSET_FROM_HERO[1],
            position[2] + SPAWN_OFFSET_FROM_HERO[2],
            position[3],
        ]);
        if self.teleport_without_the_player_flag(body, &beside, &Vec4(rotation)) {
            log_line("partner: placed the spawned body beside the hero");
        }
    }

    #[expect(unsafe_code, reason = "calls the game's teleport on the partner body")]
    fn teleport_without_the_player_flag(
        &self,
        body: usize,
        position: &Vec4,
        rotation: &Vec4,
    ) -> bool {
        let calls = &self.calls;
        // With the player flag set the game's teleport also runs its "the hero was moved" part.
        let flag_is_clear = calls
            .write_i32_at(calls.body_field(body, BODY_PLAYER_CONTROLLED_OFFSET), 0)
            .is_ok();
        if !flag_is_clear {
            return false;
        }
        // SAFETY: the caller resolved the body from its live handle or spawned it this tick.
        unsafe { calls.teleport_reset_hap(body, position, rotation) };
        true
    }

    #[expect(unsafe_code, reason = "calls the game's partner link function")]
    fn link_again(&mut self, hero_body: usize, body: usize) {
        if !self.hold_control_fields(body) {
            return;
        }
        let Ok(entity) = self.calls.entity_of_body(body) else {
            return;
        };
        if entity == 0 {
            return;
        }
        // SAFETY: the hero came from the game's getter and the body from its live handle this
        // tick; the handle field belongs to that body's entity.
        unsafe {
            self.calls
                .set_buddy_handle(hero_body, entity + ENTITY_HANDLE_OFFSET as usize);
        }
    }

    fn drive(
        &mut self,
        net: &NetThread<AutomataNet>,
        body: usize,
        now_ms: u64,
        link: &LinkView,
    ) -> bool {
        if !self.hold_control_fields(body) || !self.body_is_updating(body) {
            return false;
        }
        if self.partner_is_in_another_phase(link) {
            return false;
        }
        if self.a_jump_keeps_the_players_apart(link) {
            return false;
        }
        self.log_body_reading(body, now_ms, link);
        let fed = self.feed.stick;
        if fed {
            let feed = net.update(|shared| shared.extras.remote_input.take_feed());
            let feed = if link.fresh_remote.is_some() {
                feed
            } else {
                feed.stopped()
            };
            let feed = if self.feed.buttons {
                feed
            } else {
                feed.without_buttons()
            };
            set_input_feed(body + BODY_CONTROLLER_OFFSET as usize, feed);
        }
        let Some(remote) = link.fresh_remote else {
            if !self.logged.no_fresh_remote_state {
                self.logged.no_fresh_remote_state = true;
                log_line(
                    "partner: no fresh state from the other player yet, the body is only held",
                );
            }
            return fed;
        };
        self.show_the_owners_health(body, &remote);
        self.correct_position_and_facing(body, now_ms, &remote);
        if self.last_remote_sequence != Some(remote.sequence) {
            self.last_remote_sequence = Some(remote.sequence);
            self.apply_state_rule(body, &remote);
        }
        fed
    }

    fn log_body_reading(&mut self, body: usize, now_ms: u64, link: &LinkView) {
        let due = self
            .last_body_reading_line_ms
            .is_none_or(|last| now_ms >= last + BODY_READING_LINE_INTERVAL_MS);
        if !due || self.body_reading_lines >= BODY_READING_LINES_LOGGED_AT_MOST {
            return;
        }
        self.last_body_reading_line_ms = Some(now_ms);
        self.body_reading_lines += 1;
        let calls = &self.calls;
        let int = |offset| calls.read_i32_at(calls.body_field(body, offset));
        log_line(&format!(
            "partner: body reading {}: position {:?}, state {:X?}, health {:?}, base health {:?}, berserk switch {:?}, break fields {:X?}, hits of this game kept off its health so far {}, its Auto-Use Item checks skipped so far {}; the other player: position {:?}, state {:X?}, health {:?}, stick length {:?}",
            self.body_reading_lines,
            calls.read_body_position(body).map(|p| [p[0], p[1], p[2]]),
            int(BODY_STATE_OFFSET),
            int(BODY_HEALTH_OFFSET),
            int(BODY_BASE_HEALTH_OFFSET),
            int(BODY_BERSERK_SWITCH_OFFSET),
            calls.read_u32s_at::<BODY_BREAK_FIELD_COUNT>(
                calls.body_field(body, BODY_BREAK_FIELDS_OFFSET)
            ),
            hits_kept_off_the_partner_body(),
            auto_item_use_checks_skipped_on_the_partner_body(),
            link.fresh_remote.map(|remote| remote.position),
            link.fresh_remote.map(|remote| remote.state),
            link.fresh_remote.map(|remote| remote.health),
            link.fresh_remote.map(|remote| remote.speed),
        ));
    }

    fn a_jump_keeps_the_players_apart(&mut self, link: &LinkView) -> bool {
        let local_pending = crate::travel_driver::jump_is_pending();
        let remote_pending = link.newest_remote.is_some_and(|remote| remote.jump_pending);
        if partner_positions_apply(local_pending, remote_pending) {
            return false;
        }
        if !self.logged.jump_kept_players_apart {
            self.logged.jump_kept_players_apart = true;
            log_line(&format!(
                "partner: a jump is pending or under way (this game {local_pending}, the other player {remote_pending}); the body is only held, no position or input of the other player is applied until both are in the same place (logged once)"
            ));
        }
        true
    }

    fn partner_is_in_another_phase(&mut self, link: &LinkView) -> bool {
        let Some(remote_phase_id) = link.newest_remote.map(|remote| remote.phase_id) else {
            return false;
        };
        let phases = (self.local_phase_id, remote_phase_id);
        if phases.0 == phases.1 {
            return false;
        }
        if self.logged.phases_that_differed != Some(phases) {
            self.logged.phases_that_differed = Some(phases);
            log_line(&format!(
                "partner: this game is in phase {:#X} and the other player's in {:#X}; the body is only held, nothing is fed or corrected with coordinates of another scene",
                phases.0, phases.1
            ));
        }
        true
    }

    fn hold_control_fields(&mut self, body: usize) -> bool {
        let Ok(current) = self.calls.read_body_fields(body) else {
            return false;
        };
        let wanted = current.taken_over();
        if self
            .calls
            .write_body_fields(body, &current, &wanted)
            .is_err()
        {
            return false;
        }
        if !self.logged.fields_held {
            self.logged.fields_held = true;
            log_line(&format!(
                "partner: holding the body's control fields every tick (flags {:#X} to {:#X}, player flag 1, input sources off)",
                current.flags, wanted.flags
            ));
        }
        if wanted.is_computer_controlled() && !self.logged.still_computer_controlled {
            self.logged.still_computer_controlled = true;
            log_line(&format!(
                "partner: the body still counts as computer controlled (flags {:#X}); bit 6 is a guess and is left alone, so its controller will not run",
                wanted.flags
            ));
        }
        true
    }

    fn body_is_updating(&mut self, body: usize) -> bool {
        let calls = &self.calls;
        let suspend_flags = calls.read_u32_at(calls.body_field(body, BODY_SUSPEND_FLAGS_OFFSET));
        let update_gate = calls.read_i32_at(calls.body_field(body, BODY_UPDATE_GATE_OFFSET));
        let updating = suspend_flags.is_ok_and(|flags| flags & SUSPENDED_FLAG_BIT == 0)
            && update_gate.is_ok_and(|gate| gate < 0);
        if !updating && !self.logged.body_not_updating {
            self.logged.body_not_updating = true;
            log_line(&format!(
                "partner: the body is suspended or its update gate is closed (suspend flags {suspend_flags:X?}, gate {update_gate:?}); nothing is corrected while that holds"
            ));
        }
        updating
    }

    fn correct_position_and_facing(&mut self, body: usize, now_ms: u64, remote: &PlayerState) {
        let calls = self.calls;
        let Ok(local) = calls.read_body_position(body) else {
            return;
        };
        let local_position = [local[0], local[1], local[2]];
        match self
            .corrector
            .correct(now_ms, local_position, remote.position)
        {
            PositionCorrection::Leave => {}
            PositionCorrection::Nudge(position) => {
                let written = calls
                    .write_f32s_at(calls.body_field(body, BODY_POSITION_OFFSET), &position)
                    .is_ok();
                if written && !self.logged.position_nudged {
                    self.logged.position_nudged = true;
                    log_line(
                        "partner: first position nudge toward the other player (physics may undo plain writes: a guess)",
                    );
                }
            }
            PositionCorrection::Teleport
                if self.downed.rules.partner() == Some(DownedPhase::Downed) =>
            {
                // The game's teleport would put the body in its default pose: a downed body is
                // moved by a plain write, so it stays lying.
                let written = calls
                    .write_f32s_at(
                        calls.body_field(body, BODY_POSITION_OFFSET),
                        &remote.position,
                    )
                    .is_ok();
                if written && !self.logged.downed_body_moved_without_teleport {
                    self.logged.downed_body_moved_without_teleport = true;
                    log_line(
                        "downed: the downed other player's body was more than a teleport's distance from its owner; moved by a plain position write, not the game's teleport, so its pose is kept (logged once)",
                    );
                }
            }
            PositionCorrection::Teleport => {
                self.teleport(body, local[3], remote);
                return;
            }
        }
        let facing_address = calls.body_field(body, BODY_FACING_OFFSET);
        let needs_write = calls
            .read_f32_at(facing_address)
            .is_ok_and(|facing| facing_needs_write(facing, remote.body_facing));
        if needs_write
            && calls
                .write_f32_at(facing_address, remote.body_facing)
                .is_ok()
            && !self.logged.facing_written
        {
            self.logged.facing_written = true;
            log_line("partner: first body facing write (the angle error was large)");
        }
    }

    fn teleport(&mut self, body: usize, position_w: f32, remote: &PlayerState) {
        let calls = &self.calls;
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
        if !self.teleport_without_the_player_flag(body, &position, &rotation) {
            return;
        }
        // The game's teleport put the body in its default state, and the flag goes back to 1.
        self.state_rule.forget_started();
        self.hold_control_fields(body);
        if !self.logged.teleported {
            self.logged.teleported = true;
            log_line(&format!(
                "partner: first teleport of the body to the other player (the error was above {POSITION_ERROR_TELEPORTED})"
            ));
        }
    }

    #[expect(unsafe_code, reason = "starts a state on the held body")]
    fn apply_state_rule(&mut self, body: usize, remote: &PlayerState) {
        let calls = &self.calls;
        let Ok(local_state) = calls.read_i32_at(calls.body_field(body, BODY_STATE_OFFSET)) else {
            return;
        };
        let remote_state = RemoteState {
            state: remote.state,
            last_start: remote.last_start,
        };
        match self.state_rule.observe(local_state, remote_state) {
            StateDecision::Nothing => {}
            StateDecision::BodyNotInAPlainState(state) => {
                if self.logged.body_states_left_alone.insert(state) {
                    let break_fields = calls.read_u32s_at::<BODY_BREAK_FIELD_COUNT>(
                        calls.body_field(body, BODY_BREAK_FIELDS_OFFSET),
                    );
                    log_line(&format!(
                        "partner: the body is in state {state:#X}, not one of the plain states; no state is started on it while that holds (the other player is in {:#X}; the body's four break fields read {break_fields:X?})",
                        remote.state
                    ));
                }
            }
            StateDecision::NotAllowed(state) => {
                if self.logged.states_not_started.insert(state) {
                    log_line(&format!(
                        "partner: the other player is in state {state:#X}, which is not on the list of states started on the body; only position correction carries the body through it"
                    ));
                }
            }
            StateDecision::Start(ids) => {
                // SAFETY: the body was resolved from its live handle this tick and the rule
                // hands out only states on the allow list.
                let started = unsafe { calls.start_state(body, ids) };
                if started && !self.logged.state_started {
                    self.logged.state_started = true;
                    log_line(&format!(
                        "partner: first state start on the body (state {:#X}, arguments {}, {}, {}: {}; the body was in {local_state:#X})",
                        ids.state,
                        ids.variant,
                        ids.b,
                        ids.c,
                        if remote.last_start.is_some() {
                            "the other player's own start call"
                        } else {
                            "phase 0, the other player records no starts"
                        }
                    ));
                }
                if !started && !self.logged.other_vtable {
                    self.logged.other_vtable = true;
                    log_line(
                        "partner: the body's vtable is not the player body's, no state is started on it",
                    );
                }
            }
        }
    }

    #[expect(unsafe_code, reason = "removes a body this mod spawned")]
    fn release(
        &mut self,
        held_handle: Option<u32>,
        held_body: Option<usize>,
        restore_fields: bool,
        terminate: bool,
        give_back: GiveBack,
    ) {
        clear_input_feed();
        let saved = self.saved_fields.take();
        let Some(body) = held_body else {
            guard_held_body(None);
            return;
        };
        let calls = &self.calls;
        let possessed = calls.read_u32(POSSESSED_HANDLE_VALUE).ok();
        if held_handle.is_some_and(|handle| !gives_fields_back(handle, possessed)) {
            guard_held_body(None);
            log_line(
                "partner: the held body is the one the local player controls now (the game switched to the partner); nothing is put back on it and it is not removed",
            );
            return;
        }
        if restore_fields
            && let Some(saved) = saved
            && let Ok(current) = calls.read_body_fields(body)
        {
            let given_back = saved.given_back(current, give_back);
            let written = calls.write_body_fields(body, &current, &given_back).is_ok();
            log_line(&format!(
                "partner: put the saved fields back (flags {:#X}, player flag {}, run speed type {}, dismissed {}, written {written})",
                given_back.flags,
                given_back.player_controlled,
                given_back.run_speed_type,
                given_back.dismissed
            ));
        }
        // A live body that keeps the player flag with no guard makes three game walks endless.
        let flag_address = calls.body_field(body, BODY_PLAYER_CONTROLLED_OFFSET);
        if calls.read_i32_at(flag_address) != Ok(0) {
            let written = calls.write_i32_at(flag_address, 0).is_ok();
            log_line(&format!(
                "partner: the body's player flag was still set after the give-back, wrote 0 by itself (written {written})"
            ));
        }
        guard_held_body(None);
        if terminate
            && let Ok(entity) = calls.entity_of_body(body)
            && entity != 0
        {
            // SAFETY: the machine asks for this only for a body this mod spawned, and the body
            // was resolved from its live handle this tick.
            unsafe { calls.terminate_entity(entity) };
            log_line("partner: removed the body this mod spawned");
        }
    }
}

pub fn install(build: VerifiedBuild, settings: &GameSettings) {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        log_line("partner: image size matches no known exe, co-op is off");
        return;
    };
    let Some(port) = settings.link_port else {
        log_line("net: the launcher gave no link port, co-op is off");
        return;
    };
    let calls = GameCalls::new(build, variant);
    start_link(port, variant, settings.partner);
    install_focus_pass(calls);
    install_partner_walk_guards(calls);
    let spawn_is_guarded = install_object_read_wait_guard(calls);
    install_engine_partner_choice(calls);
    let camera_is_kept = install_camera_guard(calls);
    let pod_aim_is_carried = install_pod_aim(calls);
    let starts_are_recorded = install_start_recorder(calls);
    let feed = install_input_feed(calls, variant, settings.partner_drive);
    let own_input_is_kept = install_own_input_guard(calls, variant);
    let harm_is_kept_off = install_partner_harm_guard(calls);
    let own_item_use_is_held = install_own_item_use_hold(calls, variant);
    let pose_is_trusted = pose_handler_is_trusted(calls, variant);
    let downed_is_on =
        settings.downed_revive && harm_is_kept_off && starts_are_recorded && pose_is_trusted;
    log_line(&format!(
        "downed: downed and revive is {} (the setting says {}; the damage function is hooked: {harm_is_kept_off}, the animation start is hooked: {starts_are_recorded}, state {DOWNED_POSE_STATE:#X} is the noted blown-down handler: {pose_is_trusted}, a downed hero's own item use is held: {own_item_use_is_held}); when off, a defeat is the game's own",
        if downed_is_on { "on" } else { "off" },
        if settings.downed_revive { "on" } else { "off" }
    ));
    let drive = if feed.buttons {
        PartnerDrive::Input
    } else {
        PartnerDrive::State
    };
    log_line(&format!(
        "partner: the partner body is driven by {} (the setting says {}; every step of it is a guess until it has run); speed and facing fed: {}, the hero's own animation starts recorded: {starts_are_recorded}, camera kept from the partner body: {camera_is_kept}, local keyboard and mouse kept from the partner body: {own_input_is_kept}, pod aim sent and given to the partner's pod shot: {pod_aim_is_carried}, this game's hits kept off the partner body's health: {harm_is_kept_off}",
        drive.label(),
        settings.partner_drive.label(),
        feed.stick
    ));
    if settings.autopilot != Autopilot::Off {
        log_line(&format!(
            "autopilot: {} (a test aid: the local hero's controller gets a slow circle while the player is free)",
            settings.autopilot.label()
        ));
    }
    let Ok(mut guard) = DRIVER.lock() else {
        return;
    };
    *guard = Some(PartnerDriver {
        calls,
        local_character: settings.character,
        local_exe: variant,
        partner_setting: settings.partner,
        drive,
        feed,
        starts_are_recorded,
        spawn_is_guarded,
        requested_read: None,
        started: Instant::now(),
        machine: PartnerMachine::default(),
        saved_fields: None,
        pacer: SendPacer::default(),
        hello: HelloRule::default(),
        state_rule: StateRule::new(drive),
        corrector: PositionCorrector::default(),
        last_remote_sequence: None,
        last_hero_handle: None,
        autopilot: settings.autopilot,
        local_phase_id: 0,
        free_reason_logged: None,
        local_not_free: None,
        hero_health_log: HealthLog::of(HERO_HEALTH_LOG_NAME),
        other_player_health_log: HealthLog::of(OTHER_PLAYER_HEALTH_LOG_NAME),
        defeat_flow_log: DefeatFlowLog::default(),
        hold_log: HoldLog::default(),
        downed: DownedDriver::new(downed_is_on),
        free_change_lines: 0,
        local_held_free: HeldFree::default(),
        remote_held_free: HeldFree::default(),
        both_held_free_logged: None,
        body_reading_lines: 0,
        last_body_reading_line_ms: None,
        logged: StagesLogged::default(),
    });
}

/// The body the other player drives here, resolved from its live handle; game thread only.
pub fn held_partner_body() -> Option<usize> {
    let guard = DRIVER.try_lock().ok()?;
    let driver = guard.as_ref()?;
    let handle = driver.machine.held_body_handle()?;
    Some(driver.calls.player_body_of_handle(handle)).filter(|body| *body != 0)
}

/// The data of the character the partner body is to be, read without changing anything; `None`
/// when there is no session or a spawn could not be made safe anyway. Game thread only.
pub fn partner_data_reading() -> Option<PartnerDataReading> {
    let guard = DRIVER.try_lock().ok()?;
    guard.as_ref()?.read_partner_data_ahead()
}

/// Asks the game, without waiting, to read the character's data. False when it was not taken.
pub fn ask_for_partner_data(character: Character) -> bool {
    let Ok(mut guard) = DRIVER.try_lock() else {
        return false;
    };
    guard
        .as_mut()
        .is_some_and(|driver| driver.request_object_read(character.object_id()))
}

pub fn body_status_line() -> String {
    BODY_STATUS_LINE
        .lock()
        .map(|line| line.clone())
        .ok()
        .filter(|line| !line.is_empty())
        .unwrap_or_else(|| NO_SESSION_BODY_STATUS_TEXT.to_owned())
}

pub fn tick() {
    let Ok(mut guard) = DRIVER.try_lock() else {
        return;
    };
    let (Some(driver), Some(net)) = (guard.as_mut(), handle()) else {
        return;
    };
    driver.tick(net);
}

fn log_all(lines: Vec<String>) {
    for line in lines {
        log_line(&line);
    }
}
