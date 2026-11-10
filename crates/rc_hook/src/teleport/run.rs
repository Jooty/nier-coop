use std::time::Duration;

use rc_game::arrival::partner_shares_phase;
use rc_game::downed::DownedState;
use rc_game::marker::read_world_flags;
use rc_game::stay_together::{
    PARTNER_TIMEOUT_MS, choose_spot, distance_between, local_phase_index,
};
use rc_game::story_relay::{GatherFacts, GatherVerdict, gather_verdict};
use rc_game::{Position, Replicant};
use rc_proto::{GatherForScene, Message, TeleportHere};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::downed::downed_status;
use crate::memory::InProcessMemory;
use crate::net;
use crate::stay_together::{CollisionProbe, announced_host_in_side_view, side_view_now};
use crate::teleport::move_player_with_native;
use crate::teleport::status::{host_can_teleport, remember_outcome};
use crate::teleport::verify;

pub(crate) fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let wanted = |message: &Message| {
        matches!(
            message,
            Message::TeleportHere(_) | Message::GatherForScene(_)
        )
    };
    for message in net.take_relayed(wanted) {
        match message {
            Message::TeleportHere(here) => move_here(build, here),
            Message::GatherForScene(gather) => gather_for_scene(build, gather),
            _ => {}
        }
    }
    verify::tick(build);
}

pub(crate) fn bring_guest_here(build: &VerifiedBuild) -> Result<(), String> {
    let result = send_host_position(build);
    report(&result, "teleport: asked the guest to come to the host");
    result
}

pub(crate) fn go_to_guest(build: &VerifiedBuild) -> Result<(), String> {
    let result = move_to_guest(build);
    report(&result, "teleport: asked the game to move to the guest");
    result
}

fn report(result: &Result<(), String>, done: &str) {
    match result {
        Ok(()) => {
            log_line(done);
            remember_outcome(done);
        }
        Err(reason) => {
            log_line(&format!("teleport: refused ({reason})"));
            remember_outcome(reason);
        }
    }
}

fn send_host_position(build: &VerifiedBuild) -> Result<(), String> {
    if !host_can_teleport() {
        return Err(String::from("only a connected host can teleport"));
    }
    let net = net::handle().ok_or("not in a co-op session")?;
    let game = Replicant::new(InProcessMemory::new(build));
    let phase = local_phase_index(&game).ok_or("this game's map is not known")?;
    let position = game.position().map_err(|error| error.to_string())?;
    net.send_relayed(Message::TeleportHere(TeleportHere {
        phase,
        position: [position.x, position.y, position.z],
    }));
    Ok(())
}

fn move_to_guest(build: &VerifiedBuild) -> Result<(), String> {
    if !host_can_teleport() {
        return Err(String::from("only a connected host can teleport"));
    }
    let net = net::handle().ok_or("not in a co-op session")?;
    let view = net.view();
    let remote = view
        .remote
        .as_ref()
        .filter(|_| view.partner_in_play())
        .filter(|remote| remote.age() <= Duration::from_millis(PARTNER_TIMEOUT_MS))
        .ok_or("the guest's position is not known")?;
    let game = Replicant::new(InProcessMemory::new(build));
    let local_phase = local_phase_index(&game);
    if !partner_shares_phase(local_phase, remote.state.phase) {
        return Err(String::from("the guest is in another map"));
    }
    let [x, y, z] = remote.state.position;
    place(build, &game, Position { x, y, z })
}

fn move_here(build: &VerifiedBuild, here: TeleportHere) {
    let game = Replicant::new(InProcessMemory::new(build));
    let local_phase = local_phase_index(&game);
    if !partner_shares_phase(local_phase, here.phase) {
        log_line("teleport: the host asked you to come, but you are in another map, left alone");
        return;
    }
    let [x, y, z] = here.position;
    match place(build, &game, Position { x, y, z }) {
        Ok(()) => log_line("teleport: asked the game to move to the host"),
        Err(reason) => log_line(&format!("teleport: could not move to the host ({reason})")),
    }
}

fn gather_for_scene(build: &VerifiedBuild, gather: GatherForScene) {
    let game = Replicant::new(InProcessMemory::new(build));
    let local_phase = local_phase_index(&game);
    let flags = read_world_flags(game.memory()).ok();
    let facts = GatherFacts {
        same_phase: partner_shares_phase(local_phase, gather.phase),
        guest_in_side_view: side_view_now(build),
        host_in_side_view: announced_host_in_side_view(gather.phase),
        in_own_talk: flags.is_some_and(|flags| flags.talk_window_open),
        in_event_scene: flags.is_some_and(|flags| flags.event_scene),
        downed: downed_status().local == DownedState::Downed,
    };
    if let GatherVerdict::Stay(reason) = gather_verdict(&facts) {
        log_line(&format!("gather: staying here ({})", reason.describe()));
        return;
    }
    let [x, y, z] = gather.position;
    let target = Position { x, y, z };
    let away = game
        .position()
        .ok()
        .and_then(|local| distance_between([local.x, local.y, local.z], [x, y, z]));
    log_line(&format!(
        "gather: the host starts a scene {} units away, moving to the host",
        away.map_or_else(|| String::from("?"), |units| format!("{units:.1}"))
    ));
    if let Err(reason) = place(build, &game, target) {
        log_line(&format!("gather: could not move to the host ({reason})"));
    }
}

fn place(
    build: &VerifiedBuild,
    game: &Replicant<InProcessMemory>,
    target: Position,
) -> Result<(), String> {
    let local = game.position().map_err(|error| error.to_string())?;
    let spot = choose_spot(target, local, &CollisionProbe::new(build)).unwrap_or(target);
    let heading_degrees = move_player_with_native(build, game, spot);
    log_line(&format!(
        "teleport: _SetPlayerPosition({:.1}, {:.1}, {:.1}, heading {heading_degrees:.0})",
        spot.x, spot.y, spot.z
    ));
    Ok(())
}
