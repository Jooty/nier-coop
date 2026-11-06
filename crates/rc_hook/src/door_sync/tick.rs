use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};
use std::time::Instant;

use rc_game::door_sync::{PendingDoorStates, SnapshotSchedule, message_of_action, plan_door_state};
use rc_game::stay_together::FollowRole;
use rc_proto::{DoorChange, DoorReport, DoorState, MAX_DOORS_PER_REPORT, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::door_sync::calls::{door_of_layout_id, read_door_state, send_door_message};
use crate::door_sync::observe::{GUEST_USES, REGISTRY};
use crate::net::{self, NetThread};

static STARTED_AT: OnceLock<Instant> = OnceLock::new();
static SCHEDULE: Mutex<SnapshotSchedule> = Mutex::new(SnapshotSchedule::new());
static PENDING: LazyLock<Mutex<PendingDoorStates>> = LazyLock::new(Mutex::default);
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
        FollowRole::Guest => guest_tick(build, arrived, local_phase, now),
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
        if let Message::DoorRequest(change) = message {
            apply_change(build, change, "host opened for the guest");
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

fn guest_tick(build: &VerifiedBuild, arrived: Vec<Message>, local_phase: Option<u16>, now: u64) {
    let mut pending = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
    for message in arrived {
        match message {
            Message::DoorChange(change) => apply_change(build, change, "guest mirrors the host"),
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
    let outcome = pending.retry(local_phase, now, |state| apply_state(build, state));
    if outcome.applied > 0 || outcome.given_up > 0 {
        log_line(&format!(
            "doors: guest applied {} door states, gave up on {}",
            outcome.applied, outcome.given_up
        ));
    }
}

fn apply_change(build: &VerifiedBuild, change: DoorChange, who: &str) {
    let Some(door) = door_of_layout_id(build, change.layout_id) else {
        log_line(&format!(
            "doors: {who}: door {} is not in this zone, {:?} {} dropped",
            change.layout_id, change.action, change.flag
        ));
        return;
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
    log_line(&format!(
        "doors: guest set door {} to open {} locked {} (was open {open} locked {locked})",
        state.layout_id, state.open, state.locked
    ));
    true
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
