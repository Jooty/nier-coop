use std::time::Duration;

use rc_game::arrival::partner_shares_phase;
use rc_game::stay_together::{PARTNER_TIMEOUT_MS, choose_spot, local_phase_index};
use rc_game::{Position, Replicant};
use rc_proto::{Message, TeleportHere};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks;
use crate::memory::InProcessMemory;
use crate::net;
use crate::stay_together::CollisionProbe;
use crate::teleport::status::{host_can_teleport, remember_outcome};

pub(crate) fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    for message in net.take_relayed(|message| matches!(message, Message::TeleportHere(_))) {
        let Message::TeleportHere(here) = message else {
            continue;
        };
        move_here(build, here);
    }
}

pub(crate) fn bring_guest_here(build: &VerifiedBuild) -> Result<(), String> {
    let result = send_host_position(build);
    report(&result, "teleport: asked the guest to come to the host");
    result
}

pub(crate) fn go_to_guest(build: &VerifiedBuild) -> Result<(), String> {
    let result = move_to_guest(build);
    report(&result, "teleport: moved to the guest");
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
        Ok(()) => log_line("teleport: moved to the host"),
        Err(reason) => log_line(&format!("teleport: could not move to the host ({reason})")),
    }
}

fn place(
    build: &VerifiedBuild,
    game: &Replicant<InProcessMemory>,
    target: Position,
) -> Result<(), String> {
    if !game_hooks::teleport_available() {
        return Err(String::from("the movement hook is not installed"));
    }
    let local = game.position().map_err(|error| error.to_string())?;
    let spot = choose_spot(target, local, &CollisionProbe::new(build)).unwrap_or(target);
    game_hooks::request_teleport(spot);
    log_line(&format!(
        "teleport: moving to {:.1}, {:.1}, {:.1}",
        spot.x, spot.y, spot.z
    ));
    Ok(())
}
