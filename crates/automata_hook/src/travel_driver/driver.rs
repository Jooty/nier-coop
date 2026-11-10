use std::time::Instant;

use automata_game::address::{CURRENT_PHASE_ID_VALUE, SCENE_LOADED_FLAG_VALUE, SCENE_STATE_VALUE};
use automata_game::build::variant_of_image_size;
use automata_game::partner::{BODY_ROTATION_EULER_OFFSET, BODY_STATE_OFFSET};
use automata_game::start::PlayReadings;
use automata_game::travel::{
    GoToPartnerReadings, IncomingTeleport, JumpOrigin, JumpRole, LoadedPoints, MoveAsker,
    POINT_LIST_READ_INTERVAL_MS, TeleportRequest, TravelAction, TravelMachine, TravelPoint,
    TravelStep, TravelView, WorldReadings, decide_go_to_partner, go_to_partner_line,
    handle_teleport_request, point_list_log_line,
};
use automata_proto::Message;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;
use nc_hook::net::NetState;

use crate::game_calls::{GameCalls, Vec4};
use crate::game_hooks::expect_travel_scene_request;
use crate::net::handle;
use crate::partner_driver::held_partner_body;
use crate::travel_driver::NOT_READ_SENTINEL;
use crate::travel_driver::driver_state::{DRIVER, FirstUsesLogged, TravelDriver, VIEW};

impl TravelDriver {
    fn now_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    fn tick(&mut self) {
        let now_ms = self.now_ms();
        let role = link_role();
        self.take_partner_requests(now_ms, role);
        self.refresh_points(now_ms, false);
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
        for message in net.take_relayed(|message| matches!(message, Message::Teleport(_))) {
            let Message::Teleport(request) = message else {
                continue;
            };
            match handle_teleport_request(role, request) {
                IncomingTeleport::GoToPartner => self.go_to_partner(MoveAsker::Partner),
                IncomingTeleport::FollowJump(point) => {
                    log_all(self.machine.follow_host(now_ms, point));
                }
                ignored => {
                    if let Some(line) = ignored.ignored_line() {
                        log_line(&line);
                    }
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
        let value = |address| calls.read_i32(address).unwrap_or(NOT_READ_SENTINEL);
        WorldReadings {
            play: PlayReadings {
                scene_state: value(SCENE_STATE_VALUE),
                phase_id: value(CURRENT_PHASE_ID_VALUE),
                scene_loaded: value(SCENE_LOADED_FLAG_VALUE),
                player_present: calls.current_hero_present(),
            },
            fast_travel_state: calls.fast_travel_state(),
            fast_travel_step: calls.fast_travel_step(),
            loaded: self.loaded,
        }
    }

    fn run_step(&mut self, step: TravelStep, now_ms: u64, role: JumpRole) {
        log_all(step.logs);
        if let TravelAction::Call { point, origin } = step.action {
            self.call_transport(point, origin, now_ms, role);
        }
    }

    fn call_transport(
        &mut self,
        point: TravelPoint,
        origin: JumpOrigin,
        now_ms: u64,
        role: JumpRole,
    ) {
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
        if taken
            && origin == JumpOrigin::Local
            && role == JumpRole::Host
            && let Some(net) = handle()
        {
            net.send_relayed(Message::Teleport(TeleportRequest::follow(point)));
            log_line(&format!(
                "travel: told the guest's game to do the same jump to {} (a reliable message)",
                point.code()
            ));
        }
    }

    fn go_to_partner(&mut self, asker: MoveAsker) {
        let calls = self.calls;
        let hero = calls.current_hero();
        let partner = held_partner_body();
        let hero_state = (hero != 0)
            .then(|| {
                calls
                    .read_i32_at(calls.body_field(hero, BODY_STATE_OFFSET))
                    .ok()
            })
            .flatten();
        let decision = decide_go_to_partner(&GoToPartnerReadings {
            hero_present: hero != 0,
            hero_state,
            partner_body_present: partner.is_some(),
        });
        let outcome = match (decision, partner) {
            (Ok(()), Some(partner)) => self.move_hero_to(hero, partner),
            (Ok(()), None) => Err("there is no partner body yet".to_owned()),
            (Err(refusal), _) => Err(refusal.to_string()),
        };
        self.move_line = go_to_partner_line(asker, &outcome);
        log_line(&format!("teleport: {}", self.move_line));
    }

    #[expect(unsafe_code, reason = "calls the game's teleport on the hero")]
    fn move_hero_to(&mut self, hero: usize, partner: usize) -> Result<(), String> {
        let calls = self.calls;
        let position = calls
            .read_body_position(partner)
            .map_err(|error| format!("the partner body's position cannot be read ({error})"))?;
        let rotation = calls
            .read_f32s_at::<4>(calls.body_field(hero, BODY_ROTATION_EULER_OFFSET))
            .map_err(|error| format!("the hero's rotation cannot be read ({error})"))?;
        // Zero x and z keep the game on its branch that stores the vector as the body's rotation.
        let facing = Vec4([0.0, rotation[1], 0.0, rotation[3]]);
        // SAFETY: the hero came from the game's getter and the partner from its live handle, both
        // in this call on the game thread.
        unsafe { calls.teleport_possessed_body(hero, &Vec4(position), &facing) };
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
            travel_line: self.machine.line().to_owned(),
            move_line: self.move_line.clone(),
        };
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
        logged: FirstUsesLogged::default(),
    });
    log_line(
        "travel: the Teleport section is on: go to partner uses the game's reset teleport, the map teleporter the game's own transport call (each is a guess until its first-use line appears)",
    );
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
    with_driver(|driver| driver.go_to_partner(MoveAsker::Local));
}

fn with_driver(run: impl FnOnce(&mut TravelDriver)) {
    let Ok(mut guard) = DRIVER.try_lock() else {
        return;
    };
    if let Some(driver) = guard.as_mut() {
        run(driver);
    }
}

fn link_role() -> JumpRole {
    handle().map_or(JumpRole::Solo, |net| {
        net.update(|shared| {
            JumpRole::of_link(shared.state == NetState::Connected, shared.local_slot)
        })
    })
}

fn log_all(lines: Vec<String>) {
    for line in lines {
        log_line(&line);
    }
}
