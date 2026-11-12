use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use automata_game::build::variant_of_image_size;
use automata_game::partner::{
    BODY_ROTATION_EULER_OFFSET, BODY_STATE_OFFSET, REMOTE_STATE_STALE_MS, SPAWN_OFFSET_FROM_HERO,
};
use automata_game::start::is_in_play;
use automata_game::travel::{
    FAST_TRAVEL_LAST_STEP_BEFORE_THE_JUMP, GoToPartnerReadings, IncomingTeleport, JumpOrigin,
    JumpRole, LoadedPoints, MOVE_TO_PARTNER_MIN_INTERVAL_MS, MoveAsker,
    POINT_LIST_READ_INTERVAL_MS, POINT_LIST_WANTED_AFTER_THE_MENU_ASKED_MS, TeleportRequest,
    TravelAction, TravelMachine, TravelPoint, TravelStep, TravelView, WorldReadings,
    decide_go_to_partner, go_to_partner_line, handle_teleport_request, move_distance,
    newest_come_sender, newest_requests, point_list_log_line,
};
use automata_proto::Message;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;
use nc_hook::net::{NetState, net_clock_ms};

use crate::extra_bodies::body_of_player;
use crate::game_calls::{GameCalls, Vec4};
use crate::game_hooks::expect_travel_scene_request;
use crate::net::{SessionModeExt, handle};
use crate::partner_driver::held_partner_body;
use crate::travel_driver::driver_state::{
    DRIVER, FirstUsesLogged, JUMP_PENDING, TravelDriver, VIEW, VIEW_ASKED_AT_MS,
};

struct PartnerView {
    state_is_fresh: bool,
    phase_id: Option<i32>,
}

impl TravelDriver {
    fn now_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    fn tick(&mut self) {
        let now_ms = self.now_ms();
        let role = link_role();
        self.take_partner_requests(now_ms, role);
        let follow_waits = self.machine.has_waiting_follow();
        if follow_waits || menu_asked_for_the_view_lately() {
            // A followed call must see the list as it is now, as a local one does.
            let fresh_for_the_follow = follow_waits && is_in_play(&self.read_world().play);
            self.refresh_points(now_ms, fresh_for_the_follow);
        }
        let world = self.read_world();
        let step = self.machine.step(now_ms, &world);
        self.run_step(step, now_ms, role);
        expect_travel_scene_request(self.machine.expects_scene_request());
        self.publish(now_ms, role, world);
    }

    fn local_jump(&mut self, point: TravelPoint) {
        let now_ms = self.now_ms();
        let role = link_role();
        self.refresh_points(now_ms, true);
        let world = self.read_world();
        let step = self.machine.request_local(role, &world, point);
        self.run_step(step, now_ms, role);
    }

    fn take_partner_requests(&mut self, now_ms: u64, role: JumpRole) {
        let Some(net) = handle() else {
            return;
        };
        let received: Vec<(u8, TeleportRequest)> = net
            .take_relayed_from(|message| matches!(message, Message::Teleport(_)))
            .into_iter()
            .filter_map(|(sender, message)| match message {
                Message::Teleport(request) => Some((sender, request)),
                _ => None,
            })
            .collect();
        let requests: Vec<TeleportRequest> = received.iter().map(|(_, request)| *request).collect();
        let newest = newest_requests(&requests);
        if newest.ignored > 0 && !self.logged.older_requests_ignored {
            self.logged.older_requests_ignored = true;
            log_line(&format!(
                "travel: {} older requests of the other player arrived in the same tick and were ignored, only the newest of each kind is run (logged once)",
                newest.ignored
            ));
        }
        if let Some(sender) = newest_come_sender(&received).filter(|_| newest.come_to_sender) {
            self.go_to_partner(MoveAsker::Partner, Some(sender));
        }
        let Some(follow) = newest.follow_jump else {
            return;
        };
        match handle_teleport_request(role, follow) {
            IncomingTeleport::GoToPartner => {}
            IncomingTeleport::FollowJump(point) => {
                log_all(self.machine.follow_host(now_ms, point));
            }
            ignored => {
                let line = ignored.ignored_line();
                if let Some(line) =
                    line.filter(|line| self.logged.ignored_lines.insert(line.clone()))
                {
                    log_line(&line);
                }
            }
        }
    }

    fn refresh_points(&mut self, now_ms: u64, even_if_read_recently: bool) {
        let read_recently = self
            .points_read_at_ms
            .is_some_and(|at_ms| now_ms.saturating_sub(at_ms) < POINT_LIST_READ_INTERVAL_MS);
        if read_recently && !even_if_read_recently {
            return;
        }
        self.points_read_at_ms = Some(now_ms);
        let mut names = self.calls.travel_point_names();
        self.loaded = LoadedPoints::of_names(names.iter().map(String::as_str));
        names.sort();
        names.dedup();
        if names != self.logged_point_names {
            log_line(&point_list_log_line(&names));
            self.logged_point_names = names;
        }
    }

    fn read_world(&self) -> WorldReadings {
        let calls = &self.calls;
        WorldReadings {
            play: calls.read_play(calls.current_hero_present()),
            fast_travel_state: calls.fast_travel_state(),
            fast_travel_step: calls.fast_travel_step(),
            loaded: self.loaded,
            paused: calls.read_game_is_paused(),
        }
    }

    fn run_step(&mut self, step: TravelStep, now_ms: u64, role: JumpRole) {
        log_all(step.logs);
        if step.cancel_waiting_step {
            self.cancel_the_waiting_step();
        }
        if let Some(point) = step.tell_guest
            && role == JumpRole::Host
            && let Some(net) = handle()
        {
            net.send_relayed(Message::Teleport(TeleportRequest::follow(point)));
            log_line(&format!(
                "travel: the scene is changing, told the guest's game to do the same jump to {} (a reliable message)",
                point.code()
            ));
        }
        if let TravelAction::Call { point, origin } = step.action {
            self.call_transport(point, origin, now_ms);
        }
    }

    fn cancel_the_waiting_step(&self) {
        // Only the step that waits for the point is cancelled; the rest value makes the update skip.
        if self.calls.fast_travel_step() != Some(FAST_TRAVEL_LAST_STEP_BEFORE_THE_JUMP) {
            return;
        }
        let written = self.calls.set_fast_travel_step_to_rest();
        log_line(&format!(
            "travel: wrote the rest value to the transport's step counter (written {written}), it now reads {:?}",
            self.calls.fast_travel_step()
        ));
    }

    fn call_transport(&mut self, point: TravelPoint, origin: JumpOrigin, now_ms: u64) {
        let taken = self.calls.fast_travel_to_point(point.code());
        if !self.logged.transport_called {
            self.logged.transport_called = true;
            log_line(&format!(
                "travel: first call of the game's own transport, for {} (the fast travel entry the transporter terminal calls; that this hook may call it is a guess until run), taken: {taken}, the transport's step counter now reads {:?}",
                point.code(),
                self.calls.fast_travel_step()
            ));
        }
        log_all(self.machine.call_returned(now_ms, point, origin, taken));
        expect_travel_scene_request(self.machine.expects_scene_request());
    }

    fn go_to_partner(&mut self, asker: MoveAsker, player_slot: Option<u8>) {
        let calls = self.calls;
        let hero = calls.current_hero();
        let partner = match player_slot {
            Some(slot) => body_of_player(slot),
            None => held_partner_body(),
        };
        let hero_state = (hero != 0)
            .then(|| {
                calls
                    .read_i32_at(calls.body_field(hero, BODY_STATE_OFFSET))
                    .ok()
            })
            .flatten();
        let now_ms = self.now_ms();
        let partner_view = partner_view(player_slot);
        let local_phase_id = calls.read_play(hero != 0).phase_id;
        let decision = decide_go_to_partner(&GoToPartnerReadings {
            hero_present: hero != 0,
            hero_state,
            partner_body_present: partner.is_some(),
            local_free: calls.read_free().is_free(),
            partner_state_fresh: partner_view.state_is_fresh,
            same_phase: partner_view
                .phase_id
                .map(|phase_id| phase_id == local_phase_id),
            ms_since_last_move: self.moved_at_ms.map(|at_ms| now_ms.saturating_sub(at_ms)),
        });
        let outcome = match (decision, partner) {
            (Ok(()), Some(partner)) => self.move_hero_to(hero, partner, now_ms),
            (Ok(()), None) => Err("there is no partner body yet".to_owned()),
            (Err(refusal), _) => Err(refusal.to_string()),
        };
        let line = go_to_partner_line(asker, &outcome);
        // The same refusal, asked again and again, is one line every few seconds.
        let said_lately = line == self.move_line
            && self.move_line_logged_at_ms.is_some_and(|at_ms| {
                now_ms.saturating_sub(at_ms) < MOVE_TO_PARTNER_MIN_INTERVAL_MS
            });
        self.move_line = line;
        if !said_lately {
            self.move_line_logged_at_ms = Some(now_ms);
            log_line(&format!("teleport: {}", self.move_line));
        }
    }

    #[expect(unsafe_code, reason = "calls the game's teleport on the hero")]
    fn move_hero_to(&mut self, hero: usize, partner: usize, now_ms: u64) -> Result<(), String> {
        let calls = self.calls;
        let partner_position = calls
            .read_body_position(partner)
            .map_err(|error| format!("the partner body's position cannot be read ({error})"))?;
        let before = calls
            .read_body_position(hero)
            .map_err(|error| format!("the hero's position cannot be read ({error})"))?;
        // Beside the body, as the engine places a partner beside the hero, not inside it.
        let position = [
            partner_position[0] + SPAWN_OFFSET_FROM_HERO[0],
            partner_position[1] + SPAWN_OFFSET_FROM_HERO[1],
            partner_position[2] + SPAWN_OFFSET_FROM_HERO[2],
            partner_position[3],
        ];
        let rotation = calls
            .read_f32s_at::<4>(calls.body_field(hero, BODY_ROTATION_EULER_OFFSET))
            .map_err(|error| format!("the hero's rotation cannot be read ({error})"))?;
        // Zero x and z keep the game on its branch that stores the vector as the body's rotation.
        let facing = Vec4([0.0, rotation[1], 0.0, rotation[3]]);
        // SAFETY: the hero came from the game's getter and the partner from its live handle, both
        // in this call on the game thread.
        unsafe { calls.teleport_possessed_body(hero, &Vec4(position), &facing) };
        self.moved_at_ms = Some(now_ms);
        log_line(&format!(
            "teleport: moved the hero {:.1} units to the partner body",
            move_distance(
                [before[0], before[1], before[2]],
                [position[0], position[1], position[2]]
            )
        ));
        if !self.logged.hero_moved {
            self.logged.hero_moved = true;
            log_line(&format!(
                "teleport: first move of the hero to the partner body with the game's reset teleport (camera argument 1; a guess until run): target {:?}, the hero now reads {:?}",
                &position[..3],
                calls
                    .read_body_position(hero)
                    .map(|after| [after[0], after[1], after[2]])
                    .ok()
            ));
        }
        Ok(())
    }

    fn publish(&self, now_ms: u64, role: JumpRole, world: WorldReadings) {
        let view = TravelView {
            role,
            world: Some(world),
            jump_under_way: self.machine.is_under_way(),
            current: self.machine.current_point(now_ms),
            partner_body_present: held_partner_body().is_some(),
            same_phase_as_partner: partner_view(None)
                .phase_id
                .map(|phase_id| phase_id == world.play.phase_id),
            travel_line: self.machine.line().to_owned(),
            move_line: self.move_line.clone(),
        };
        JUMP_PENDING.store(self.machine.jump_is_pending(), Ordering::Relaxed);
        if let Ok(mut shown) = VIEW.lock() {
            *shown = Some(view);
        }
    }
}

pub fn install(build: VerifiedBuild) {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        log_line("travel: image size matches no known exe, the Teleport section stays off");
        return;
    };
    let Ok(mut guard) = DRIVER.lock() else {
        return;
    };
    *guard = Some(TravelDriver {
        calls: GameCalls::new(build, variant),
        started: Instant::now(),
        machine: TravelMachine::default(),
        loaded: LoadedPoints::default(),
        points_read_at_ms: None,
        logged_point_names: Vec::new(),
        move_line: String::new(),
        move_line_logged_at_ms: None,
        moved_at_ms: None,
        logged: FirstUsesLogged::default(),
    });
    log_line(
        "travel: the Teleport section is on: go to partner uses the game's reset teleport, the map teleporter the game's own transport call (each is a guess until its first-use line appears)",
    );
}

pub fn jump_is_pending() -> bool {
    JUMP_PENDING.load(Ordering::Relaxed)
}

pub fn tick() {
    with_driver(TravelDriver::tick);
}

/// Game thread only: the queued menu action.
pub fn run_local_jump(point: TravelPoint) {
    with_driver(|driver| driver.local_jump(point));
}

/// Game thread only: the queued menu action.
pub fn run_go_to_partner() {
    with_driver(|driver| driver.go_to_partner(MoveAsker::Local, None));
}

pub fn run_go_to_player(slot: u8) {
    with_driver(|driver| driver.go_to_partner(MoveAsker::Local, Some(slot)));
}

fn with_driver(run: impl FnOnce(&mut TravelDriver)) {
    let Ok(mut guard) = DRIVER.try_lock() else {
        return;
    };
    if let Some(driver) = guard.as_mut() {
        run(driver);
    }
}

fn menu_asked_for_the_view_lately() -> bool {
    let asked_at_ms = VIEW_ASKED_AT_MS.load(Ordering::Relaxed);
    asked_at_ms != 0
        && net_clock_ms().saturating_sub(asked_at_ms) <= POINT_LIST_WANTED_AFTER_THE_MENU_ASKED_MS
}

fn partner_view(player_slot: Option<u8>) -> PartnerView {
    let unknown = PartnerView {
        state_is_fresh: false,
        phase_id: None,
    };
    handle().map_or(unknown, |net| {
        net.update(|shared| {
            let player = match player_slot {
                Some(slot) => shared.player(slot),
                None => shared.companion_player(),
            };
            let remote = player.and_then(|player| player.latest.as_ref());
            PartnerView {
                state_is_fresh: remote.is_some_and(|remote| {
                    remote.age() <= Duration::from_millis(REMOTE_STATE_STALE_MS)
                }),
                phase_id: remote.map(|remote| remote.state.phase_id),
            }
        })
    })
}

fn link_role() -> JumpRole {
    handle().map_or(JumpRole::Solo, |net| {
        net.update(|shared| {
            JumpRole::of_session(
                shared.session_mode(),
                shared.state == NetState::Connected,
                shared.local_slot,
            )
        })
    })
}

fn log_all(lines: Vec<String>) {
    for line in lines {
        log_line(&line);
    }
}
