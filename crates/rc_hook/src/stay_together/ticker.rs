use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;

use rc_game::fix_buttons::{FixButton, host_target_for_session, rejoin_target};
use rc_game::stay_together::{
    AnnounceConditions, FollowConditions, FollowRole, FollowState, FollowStep, HostAnnouncer,
    PhaseTarget, announced_target, check_partner_reachable, choose_spot, local_phase_index,
};
use rc_game::{PHASE_TABLE, Position, Replicant};
use rc_proto::{FollowPhase, Message, PartnerNotice};

use crate::actions::{self, Action};
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::event_log::post_event;
use crate::game_hooks;
use crate::memory::InProcessMemory;
use crate::net::{self, NetShared, NetState};
use crate::stay_together::arrival::{ArrivalState, keep_safe};
use crate::stay_together::free_to_act::game_says_free_to_act;
use crate::stay_together::menu_close::{close_main_menu, main_menu_open};
use crate::stay_together::settings::{
    clear_pending_requests, follow_enabled, take_go_to_partner_request,
    take_rejoin_host_area_request, update_status,
};
use crate::stay_together::world_probe::CollisionProbe;

static STARTED_AT: OnceLock<Instant> = OnceLock::new();
static TICKER: Mutex<TickerState> = Mutex::new(TickerState::new());

struct TickerState {
    follow: FollowState,
    announcer: HostAnnouncer,
    arrival: ArrivalState,
    last_host_target: Option<PhaseTarget>,
}

impl TickerState {
    const fn new() -> Self {
        Self {
            follow: FollowState::new(),
            announcer: HostAnnouncer::new(),
            arrival: ArrivalState::new(),
            last_host_target: None,
        }
    }
}

pub fn tick(build: &VerifiedBuild) {
    let mut state = TICKER.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(net) = net::handle() else {
        state.last_host_target = None;
        clear_pending_requests();
        return;
    };
    let view = net.view();
    let connected_to_peer = view.state == NetState::Connected;
    state.last_host_target = host_target_for_session(state.last_host_target, connected_to_peer);
    if !connected_to_peer {
        clear_pending_requests();
    }
    let now_ms = STARTED_AT.get_or_init(Instant::now).elapsed().as_millis() as u64;
    let game = Replicant::new(InProcessMemory::new(build));
    let local_phase = local_phase_index(&game);
    let role = FollowRole::from_slot(view.local_slot);
    let connected = view.partner_in_play();
    game_hooks::update_menu_no_pause(build, connected);
    crate::paused_hits::tick(net, &view);
    announce_as_host(&mut state, &game, net, role, connected, now_ms);
    if let Some(Message::FollowPhase(follow)) = net
        .take_relayed(|message| matches!(message, Message::FollowPhase(_)))
        .pop()
        && usize::from(follow.phase) < PHASE_TABLE.len()
    {
        let target = PhaseTarget {
            phase: follow.phase,
            point: follow.point,
        };
        state.last_host_target = Some(target);
        state.follow.host_moved_to(target);
    }
    follow_the_host(&mut state, build, role, connected, local_phase, now_ms);
    keep_safe(&mut state.arrival, build, &game, &view, local_phase, now_ms);
    if take_go_to_partner_request() {
        let outcome = go_to_partner(build, &game, &view, local_phase);
        update_status(|status| status.go_to_partner = outcome);
    }
    if take_rejoin_host_area_request() {
        let outcome = rejoin_host_area(&mut state, build, net, &view, now_ms);
        update_status(|status| status.rejoin = outcome);
    }
}

fn announce_as_host(
    state: &mut TickerState,
    game: &Replicant<InProcessMemory>,
    net: &net::NetThread,
    role: FollowRole,
    connected: bool,
    now_ms: u64,
) {
    let conditions = AnnounceConditions {
        role,
        connected,
        current: announced_target(game),
        now_ms,
    };
    if let Some(target) = state.announcer.poll(&conditions) {
        net.send_relayed(Message::FollowPhase(FollowPhase {
            phase: target.phase,
            point: target.point,
        }));
    }
}

fn follow_the_host(
    state: &mut TickerState,
    build: &VerifiedBuild,
    role: FollowRole,
    connected: bool,
    local_phase: Option<u16>,
    now_ms: u64,
) {
    let conditions = FollowConditions {
        enabled: follow_enabled(),
        role,
        connected,
        free_to_act: game_says_free_to_act(build).unwrap_or(false)
            || crate::fade_sync::guest_holds_fade(),
        local_phase,
        now_ms,
    };
    let label = match state.follow.step(&conditions) {
        FollowStep::Idle => String::new(),
        FollowStep::Waiting(reason) => format!("following the host: {}", reason.label()),
        FollowStep::Go(target) => {
            announce_follow(target);
            if main_menu_open(build) {
                close_main_menu(build, "before following the host to a new zone");
            }
            state.arrival.arrival.arm(target, now_ms);
            queue_phase_change(target);
            format!("following the host: changing map (point {})", target.point)
        }
    };
    update_status(|status| status.follow = label);
}

fn announce_follow(target: PhaseTarget) {
    let Some(entry) = PHASE_TABLE.get(usize::from(target.phase)) else {
        return;
    };
    let host = net::handle()
        .and_then(|net| net.view().peer_name)
        .unwrap_or_else(|| "the host".to_owned());
    post_event(&format!("Following {host} to {}", entry.area));
}

fn queue_phase_change(target: PhaseTarget) {
    let Some(entry) = PHASE_TABLE.get(usize::from(target.phase)) else {
        return;
    };
    match Action::change_phase(entry.name, target.point) {
        Ok(action) => {
            if let Err(error) = actions::enqueue(action) {
                log_line(&format!(
                    "stay together: cannot queue the map change ({error})"
                ));
            }
        }
        Err(error) => log_line(&format!("stay together: bad map change ({error})")),
    }
}

fn rejoin_host_area(
    state: &mut TickerState,
    build: &VerifiedBuild,
    net: &net::NetThread,
    view: &NetShared,
    now_ms: u64,
) -> String {
    let id = FixButton::RejoinHostArea.log_id();
    if FollowRole::from_slot(view.local_slot) != FollowRole::Guest || !view.partner_in_play() {
        log_line(&format!("fix button {id}: refused, not a connected guest"));
        return "not rejoined: you need to be the guest with the host in play".to_owned();
    }
    let partner_phase = view.remote.as_ref().map(|remote| remote.state.phase);
    let free_to_act = game_says_free_to_act(build).unwrap_or(false)
        || crate::fade_sync::guest_holds_fade()
        || main_menu_open(build);
    let target = match rejoin_target(state.last_host_target, partner_phase, free_to_act) {
        Ok(target) => target,
        Err(refusal) => {
            log_line(&format!("fix button {id}: refused, {}", refusal.label()));
            return format!("not rejoined: {}", refusal.label());
        }
    };
    if main_menu_open(build) {
        close_main_menu(build, "before rejoining the host's area");
    }
    state.arrival.arrival.arm(target, now_ms);
    state.follow.change_queued_by_rejoin(target, now_ms);
    queue_phase_change(target);
    net.send_relayed(Message::Notice(PartnerNotice::RejoiningYourArea));
    log_line(&format!(
        "fix button {id}: reloading phase {} point {}",
        target.phase, target.point
    ));
    format!("rejoining the host's area (point {})", target.point)
}

fn go_to_partner(
    build: &VerifiedBuild,
    game: &Replicant<InProcessMemory>,
    view: &NetShared,
    local_phase: Option<u16>,
) -> String {
    let remote = view.remote.as_ref();
    let age_ms = remote.map(|remote| remote.age().as_millis() as u64);
    let partner_phase = remote.map(|remote| remote.state.phase);
    if let Err(refusal) =
        check_partner_reachable(view.partner_in_play(), age_ms, local_phase, partner_phase)
    {
        return format!("not moved: {}", refusal.label());
    }
    let (Some(remote), Ok(local)) = (remote, game.position()) else {
        return "not moved: your position is not readable".to_owned();
    };
    if !game_hooks::teleport_available() {
        return "not moved: the movement hook is not installed".to_owned();
    }
    let [x, y, z] = remote.state.position;
    let probe = CollisionProbe::new(build);
    match choose_spot(Position { x, y, z }, local, &probe) {
        Ok(spot) => {
            game_hooks::request_teleport(spot);
            format!("moving to {:.1}, {:.1}, {:.1}", spot.x, spot.y, spot.z)
        }
        Err(refusal) => format!("not moved: {}", refusal.label()),
    }
}
