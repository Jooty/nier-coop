use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};
use std::time::Instant;

use rc_game::door_sync::{
    PendingDoorStates, SnapshotSchedule, changes_for_resulting_state, door_reached_state,
    message_of_action, plan_door_state, sender_phase_matches_local,
};
use rc_game::fight_doors::HeldLocks;
use rc_game::stay_together::{FollowRole, distance_between};
use rc_proto::{DoorAction, DoorChange, DoorReport, DoorState, MAX_DOORS_PER_REPORT, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::door_sync::calls::{
    door_of_layout_id, door_position, read_door_state, send_door_message,
};
use crate::door_sync::observe::{GUEST_USES, REGISTRY};
use crate::net::{self, NetThread};

static STARTED_AT: OnceLock<Instant> = OnceLock::new();
static SCHEDULE: Mutex<SnapshotSchedule> = Mutex::new(SnapshotSchedule::new());
static PENDING: LazyLock<Mutex<PendingDoorStates>> = LazyLock::new(Mutex::default);
static HELD_LOCKS: LazyLock<Mutex<HeldLocks>> = LazyLock::new(Mutex::default);
static LAST_HOST_PHASE: Mutex<Option<u16>> = Mutex::new(None);

pub(super) fn now_ms() -> u64 {
    u64::try_from(STARTED_AT.get_or_init(Instant::now).elapsed().as_millis()).unwrap_or(u64::MAX)
}

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let role = FollowRole::from_slot(view.local_slot);
    let local_phase = view.local.as_ref().map(|local| local.phase);
    forget_doors_when_the_zone_changes(local_phase);
    let arrived = net.take_relayed(|message| {
        matches!(
            message,
            Message::DoorChange(_) | Message::DoorRequest(_) | Message::DoorReport(_)
        )
    });
    let now = now_ms();
    match role {
        FollowRole::Host => {
            let partner_phase = view
                .partner_in_play()
                .then(|| view.remote.as_ref().map(|remote| remote.state.phase))
                .flatten();
            host_tick(build, net, arrived, (local_phase, partner_phase), now);
        }
        FollowRole::Guest => {
            let local_position = view.local.as_ref().map(|local| local.position);
            guest_tick(build, arrived, (local_phase, local_position), now);
        }
        FollowRole::Unknown => {}
    }
}

fn forget_doors_when_the_zone_changes(local_phase: Option<u16>) {
    let mut last = LAST_HOST_PHASE
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if *last == local_phase {
        return;
    }
    *last = local_phase;
    REGISTRY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    GUEST_USES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    HELD_LOCKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    log_line(&format!(
        "doors: zone is now {local_phase:?}, door list cleared"
    ));
}

fn host_tick(
    build: &VerifiedBuild,
    net: &NetThread,
    arrived: Vec<Message>,
    (local_phase, partner_phase): (Option<u16>, Option<u16>),
    now: u64,
) {
    for message in arrived {
        if let Message::DoorRequest(change) = message
            && apply_change(build, change, local_phase, "host opened for the guest")
        {
            echo_resulting_state(build, net, change, local_phase);
        }
    }
    let due = {
        let mut schedule = SCHEDULE.lock().unwrap_or_else(PoisonError::into_inner);
        schedule.observe(local_phase, partner_phase, now);
        schedule.take_due(now)
    };
    if let Some(phase) = due {
        send_door_states(build, net, phase);
    }
}

fn guest_tick(
    build: &VerifiedBuild,
    arrived: Vec<Message>,
    (local_phase, local_position): (Option<u16>, Option<[f32; 3]>),
    now: u64,
) {
    let mut pending = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
    for message in arrived {
        match message {
            Message::DoorChange(change) => {
                mirror_host_change(build, change, (local_phase, local_position));
            }
            Message::DoorReport(report) => {
                log_line(&format!(
                    "doors: guest got {} door states for zone {}",
                    report.doors.len(),
                    report.phase
                ));
                pending.receive(report.phase, &report.doors, now);
            }
            _ => {}
        }
    }
    release_held_locks_left_behind(build, local_phase, local_position);
    let outcome = pending.retry(local_phase, now, |state| apply_state(build, state));
    if outcome.applied > 0 || outcome.given_up > 0 {
        log_line(&format!(
            "doors: guest applied {} door states, gave up on {}",
            outcome.applied, outcome.given_up
        ));
    }
}

fn mirror_host_change(
    build: &VerifiedBuild,
    change: DoorChange,
    (local_phase, local_position): (Option<u16>, Option<[f32; 3]>),
) {
    if change.action == DoorAction::Lock && !change.flag {
        HELD_LOCKS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .drop_hold(change.layout_id);
    }
    let locks_in_a_door_the_guest_stands_at = change.action == DoorAction::Lock
        && change.flag
        && sender_phase_matches_local(change.phase, local_phase)
        && HeldLocks::holds_a_lock_near(distance_to_door(build, change.layout_id, local_position));
    if locks_in_a_door_the_guest_stands_at {
        HELD_LOCKS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .hold(change.layout_id);
        log_line(&format!(
            "doors: guest stands at door {}, holding the host's lock until they leave",
            change.layout_id
        ));
        return;
    }
    apply_change(build, change, local_phase, "guest mirrors the host");
}

fn release_held_locks_left_behind(
    build: &VerifiedBuild,
    local_phase: Option<u16>,
    local_position: Option<[f32; 3]>,
) {
    let Some(phase) = local_phase else {
        return;
    };
    let left = HELD_LOCKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .release_those_left_behind(|layout_id| distance_to_door(build, layout_id, local_position));
    for layout_id in left {
        let change = DoorChange {
            layout_id,
            action: DoorAction::Lock,
            flag: true,
            phase,
        };
        apply_change(build, change, local_phase, "guest left the door");
    }
}

fn distance_to_door(
    build: &VerifiedBuild,
    layout_id: u16,
    local_position: Option<[f32; 3]>,
) -> Option<f32> {
    let door = door_of_layout_id(build, layout_id)?;
    distance_between(local_position?, door_position(build, door)?)
}

fn apply_change(
    build: &VerifiedBuild,
    change: DoorChange,
    local_phase: Option<u16>,
    who: &str,
) -> bool {
    if !sender_phase_matches_local(change.phase, local_phase) {
        log_line(&format!(
            "doors: {who}: door {} {:?} {} is for zone {}, this zone is {local_phase:?}, dropped",
            change.layout_id, change.action, change.flag, change.phase
        ));
        return false;
    }
    let Some(door) = door_of_layout_id(build, change.layout_id) else {
        log_line(&format!(
            "doors: {who}: door {} is not in this zone, {:?} {} dropped",
            change.layout_id, change.action, change.flag
        ));
        return false;
    };
    send_door_message(
        build,
        door,
        message_of_action(change.action),
        i64::from(change.flag),
    );
    log_line(&format!(
        "doors: {who}: door {} got {:?} {}",
        change.layout_id, change.action, change.flag
    ));
    true
}

fn echo_resulting_state(
    build: &VerifiedBuild,
    net: &NetThread,
    request: DoorChange,
    local_phase: Option<u16>,
) {
    let Some(phase) = local_phase else {
        return;
    };
    let Some((open, locked)) =
        door_of_layout_id(build, request.layout_id).and_then(|door| read_door_state(build, door))
    else {
        return;
    };
    let state = DoorState {
        layout_id: request.layout_id,
        open,
        locked,
    };
    for change in changes_for_resulting_state(phase, &state) {
        net.send_relayed(Message::DoorChange(change));
    }
    log_line(&format!(
        "doors: host tells everyone door {} is now open {open} locked {locked}",
        request.layout_id
    ));
}

fn apply_state(build: &VerifiedBuild, state: &DoorState) -> bool {
    let Some(door) = door_of_layout_id(build, state.layout_id) else {
        return false;
    };
    let Some((open, locked)) = read_door_state(build, door) else {
        return false;
    };
    for (action, flag) in plan_door_state(open, locked, state) {
        send_door_message(build, door, message_of_action(action), i64::from(flag));
    }
    let reached = read_door_state(build, door)
        .is_some_and(|(now_open, now_locked)| door_reached_state(now_open, now_locked, state));
    log_line(&format!(
        "doors: guest set door {} to open {} locked {} (was open {open} locked {locked}), reached {reached}",
        state.layout_id, state.open, state.locked
    ));
    reached
}

fn send_door_states(build: &VerifiedBuild, net: &NetThread, phase: u16) {
    let ids = REGISTRY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .ids();
    let doors: Vec<DoorState> = ids
        .into_iter()
        .filter_map(|layout_id| {
            let door = door_of_layout_id(build, layout_id)?;
            let (open, locked) = read_door_state(build, door)?;
            Some(DoorState {
                layout_id,
                open,
                locked,
            })
        })
        .collect();
    log_line(&format!(
        "doors: host sends {} door states for zone {phase}",
        doors.len()
    ));
    for chunk in doors.chunks(MAX_DOORS_PER_REPORT) {
        net.send_relayed(Message::DoorReport(DoorReport {
            phase,
            doors: chunk.to_vec(),
        }));
    }
}
