use std::time::{Duration, Instant};

use automata_game::address::POSSESSED_HANDLE_VALUE;
use automata_game::build::variant_of_image_size;
use automata_game::character::Character;
use automata_game::partner::{
    BODY_CONTROLLER_OFFSET, BODY_FACING_OFFSET, BODY_FLAGS_OFFSET, BODY_MODEL_ID_OFFSET,
    BODY_PARTNER_MARK_WORD_OFFSET, BODY_PLAYER_CONTROLLED_OFFSET, BODY_POSITION_OFFSET,
    BODY_ROTATION_EULER_OFFSET, BODY_STATE_OFFSET, BODY_SUSPEND_FLAGS_OFFSET,
    BODY_UPDATE_GATE_OFFSET, BodyFields, BuddyReading, COMPUTER_CONTROLLED_FLAG_BITS,
    ENTITY_HANDLE_OFFSET, HelloRule, HeroReading, PARTNER_MARK_BIT, POSITION_ERROR_TELEPORTED,
    PartnerAction, PartnerDrive, PartnerMachine, PartnerReadings, PositionCorrection,
    PositionCorrector, REMOTE_STATE_STALE_MS, RemoteStateIds, SPAWN_OFFSET_FROM_HERO,
    SPAWNED_BODY_NAME, SUSPENDED_FLAG_BIT, SendPacer, StateDecision, StateRule, facing_needs_write,
    wanted_partner,
};
use automata_proto::{CharacterHello, Message, PlayerState};
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;
use nc_hook::net::{NetState, NetThread};

use crate::game_calls::{GameCalls, Vec4};
use crate::game_hooks::{
    clear_input_feed, guard_held_body, install_destroy_guard, install_input_feed, set_input_feed,
};
use crate::net::{AutomataNet, handle, start_link};
use crate::partner_driver::driver_state::{DRIVER, PartnerDriver, StagesLogged};
use crate::start_setting::GameSettings;

struct LinkView {
    connected: bool,
    told: Option<Character>,
    fresh_remote: Option<PlayerState>,
}

#[derive(Clone, Copy)]
struct Hero {
    body: usize,
    reading: HeroReading,
}

impl PartnerDriver {
    fn tick(&mut self, net: &NetThread<AutomataNet>) {
        let now_ms = self.started.elapsed().as_millis() as u64;
        let link = net.update(|shared| LinkView {
            connected: shared.state == NetState::Connected,
            told: shared.extras.partner_character,
            fresh_remote: shared
                .remote
                .as_ref()
                .filter(|remote| remote.age() <= Duration::from_millis(REMOTE_STATE_STALE_MS))
                .map(|remote| remote.state),
        });
        let hero = self.read_hero();
        self.tell_character(net, &link, hero);
        self.publish_local_state(net, now_ms, hero);
        let fed = self.step_partner(net, now_ms, &link, hero);
        if !fed {
            clear_input_feed();
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
        Some(Hero {
            body,
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
        let hero_handle = hero.map(|hero| hero.reading.handle);
        if self.hello.should_send(link.connected, hero_handle) {
            net.send_relayed(Message::Character(CharacterHello {
                character: self.local_character,
            }));
            log_line(&format!(
                "net: told the other player our character ({})",
                self.local_character.launcher_id()
            ));
        }
    }

    fn publish_local_state(
        &mut self,
        net: &NetThread<AutomataNet>,
        now_ms: u64,
        hero: Option<Hero>,
    ) {
        let Some(hero) = hero else {
            return;
        };
        if !net.wants_snapshots() {
            return;
        }
        let read = self
            .calls
            .read_pressed_buttons(hero.body)
            .map(|pressed| self.pacer.observe(now_ms, pressed))
            .and_then(|due| {
                due.map(|pressed| self.calls.read_body_snapshot(hero.body, pressed))
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
    ) -> bool {
        if !link.connected && self.machine.held_body_handle().is_none() {
            return false;
        }
        let calls = self.calls;
        let held_body = self
            .machine
            .held_body_handle()
            .map(|handle| calls.player_body_of_handle(handle))
            .filter(|body| *body != 0);
        let buddy_body = hero
            .map(|_| calls.current_buddy())
            .filter(|body| *body != 0);
        let readings = PartnerReadings {
            partner_connected: link.connected,
            wanted: wanted_partner(self.partner_setting, link.told),
            hero: hero.map(|hero| hero.reading),
            buddy: buddy_body.and_then(|body| self.read_buddy(body)),
            held_object_id: held_body.and_then(|body| self.read_model_id(body)),
        };
        let step = self.machine.step(now_ms, &readings);
        log_all(step.logs);
        match step.action {
            PartnerAction::Nothing => {}
            PartnerAction::Adopt => {
                if let Some(body) = buddy_body {
                    self.take_over(body);
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
            PartnerAction::Relink => {
                if let (Some(hero), Some(body)) = (hero, held_body) {
                    self.link_again(hero.body, body);
                }
            }
            PartnerAction::Release {
                restore_fields,
                terminate,
            } => self.release(held_body, restore_fields, terminate),
        }
        false
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
                log_line(&format!(
                    "partner: saved the body's fields before taking it over: flags {:#X}, player flag {}, enables {}/{}/{}, input ignore {}, run speed type {}, dismissed {}, health floor {}; partner mark bit {partner_mark} is left as it is",
                    fields.flags,
                    fields.player_controlled,
                    fields.pad_enable,
                    fields.script_enable,
                    fields.computer_enable,
                    fields.input_ignore_seconds,
                    fields.run_speed_type,
                    fields.dismissed,
                    fields.health_floor_switch
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
        let mut name = SPAWNED_BODY_NAME.as_bytes().to_vec();
        name.push(0);
        let entity = calls.spawn_by_name(&name, object_id);
        if entity == 0 {
            log_all(
                self.machine
                    .spawn_failed("the spawn returned 0, the entity table may be full"),
            );
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

    #[expect(unsafe_code, reason = "calls the game's teleport on the spawned body")]
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
        // SAFETY: the body is the live player body spawned and linked this tick.
        unsafe { calls.teleport_reset_hap(body, &beside, &Vec4(rotation)) };
        log_line("partner: placed the spawned body beside the hero");
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
        let fed = self.input_is_fed;
        if fed {
            let feed = net.update(|shared| shared.extras.remote_input.take_feed());
            let feed = if link.fresh_remote.is_some() {
                feed
            } else {
                feed.stopped()
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
        self.correct_position_and_facing(body, now_ms, &remote);
        if self.last_remote_sequence != Some(remote.sequence) {
            self.last_remote_sequence = Some(remote.sequence);
            self.apply_state_rule(body, &remote);
        }
        fed
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
                "partner: holding the body's control fields every tick (flags {:#X} to {:#X}, player flag 1, input sources off, health floor on)",
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

    #[expect(unsafe_code, reason = "calls the game's teleport on the held body")]
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
        // SAFETY: the body was resolved from its live handle this tick.
        unsafe { calls.teleport_reset_hap(body, &position, &rotation) };
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
        let ids = RemoteStateIds {
            state: remote.state,
            variant: remote.variant,
            b: remote.b,
            c: remote.c,
        };
        match self.state_rule.observe(local_state, ids) {
            StateDecision::Nothing => {}
            StateDecision::Blocked(state) => {
                if self.logged.blocked_states.insert(state) {
                    log_line(&format!(
                        "partner: the other player is in state {state:#X}, which is never started on the body"
                    ));
                }
            }
            StateDecision::Start(ids) => {
                // SAFETY: the body was resolved from its live handle this tick and the rule
                // never hands out a never-started state.
                let started = unsafe { calls.start_state(body, ids) };
                if started && !self.logged.state_started {
                    self.logged.state_started = true;
                    log_line(&format!(
                        "partner: first state start on the body (state {:#X}, variant {}, the body was in {local_state:#X})",
                        ids.state, ids.variant
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
    fn release(&mut self, held_body: Option<usize>, restore_fields: bool, terminate: bool) {
        guard_held_body(None);
        let saved = self.saved_fields.take();
        let Some(body) = held_body else {
            return;
        };
        let calls = &self.calls;
        if restore_fields
            && let Some(saved) = saved
            && let Ok(current) = calls.read_body_fields(body)
        {
            // A dismissal the engine started stays as the engine set it.
            let given_back = BodyFields {
                dismissed: current.dismissed,
                ..saved
            };
            let written = calls.write_body_fields(body, &current, &given_back).is_ok();
            log_line(&format!(
                "partner: put the saved fields back (flags {:#X}, player flag {}, written {written})",
                saved.flags, saved.player_controlled
            ));
        }
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
    install_destroy_guard(calls);
    let input_is_fed =
        settings.partner_drive == PartnerDrive::Input && install_input_feed(calls, variant);
    let drive = if input_is_fed {
        PartnerDrive::Input
    } else {
        PartnerDrive::State
    };
    log_line(&format!(
        "partner: the partner body is driven by {} (the setting says {}; every step of it is a guess until it has run)",
        drive.label(),
        settings.partner_drive.label()
    ));
    let Ok(mut guard) = DRIVER.lock() else {
        return;
    };
    *guard = Some(PartnerDriver {
        calls,
        local_character: settings.character,
        partner_setting: settings.partner,
        drive,
        input_is_fed,
        started: Instant::now(),
        machine: PartnerMachine::default(),
        saved_fields: None,
        pacer: SendPacer::default(),
        hello: HelloRule::default(),
        state_rule: StateRule::new(drive),
        corrector: PositionCorrector::default(),
        last_remote_sequence: None,
        logged: StagesLogged::default(),
    });
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
