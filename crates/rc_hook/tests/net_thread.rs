use std::thread;
use std::time::{Duration, Instant};

use rc_hook::net::{LocalSnapshot, NetCommand, NetState, NetThread};
use rc_net::{IrohConfig, IrohTransport, LoopbackNetwork};
use rc_proto::{BuildGate, RejectReason};

const BUILD: BuildGate = BuildGate {
    time_date_stamp: 0x60B7_3C00,
    size_of_image: 0x0533_9000,
};
const OTHER_BUILD: BuildGate = BuildGate {
    time_date_stamp: 1,
    size_of_image: 2,
};
const WAIT_LIMIT: Duration = Duration::from_secs(20);

fn spawn(network: &LoopbackNetwork, name: &str, build: BuildGate) -> NetThread {
    let network = network.clone();
    NetThread::spawn(move || Ok(network.endpoint()), name.to_owned(), build)
        .expect("net thread starts")
}

fn wait_until(mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + WAIT_LIMIT;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        thread::sleep(Duration::from_millis(2));
    }
    false
}

fn snapshot_at(x: f32) -> LocalSnapshot {
    LocalSnapshot {
        phase: 3,
        position: [x, 0.0, 0.0],
        yaw_radians: 0.0,
        hp: 10,
        max_hp: 10,
        body_motion_index: None,
        body_motion_frame: 0.0,
    }
}

fn connect(host: &NetThread, guest: &NetThread) {
    host.send_command(NetCommand::Host);
    assert!(
        wait_until(|| host.view().ticket.is_some()),
        "host shows a ticket"
    );
    let ticket = host.view().ticket.expect("ticket was just seen");
    guest.send_command(NetCommand::Join(ticket));
    assert!(wait_until(|| {
        host.view().state == NetState::Connected && guest.view().state == NetState::Connected
    }));
}

#[test]
fn host_and_guest_see_each_others_names() {
    let network = LoopbackNetwork::new();
    let host = spawn(&network, "Hosty", BUILD);
    let guest = spawn(&network, "Guesty", BUILD);
    connect(&host, &guest);
    assert_eq!(host.view().peer_name.as_deref(), Some("Guesty"));
    assert_eq!(guest.view().peer_name.as_deref(), Some("Hosty"));
}

#[test]
fn published_snapshots_reach_the_other_players_store() {
    let network = LoopbackNetwork::new();
    let host = spawn(&network, "Hosty", BUILD);
    let guest = spawn(&network, "Guesty", BUILD);
    connect(&host, &guest);
    assert!(host.wants_snapshots() && guest.wants_snapshots());
    let seen = wait_until(|| {
        guest.publish(snapshot_at(42.0));
        host.view()
            .remote
            .is_some_and(|remote| remote.state.position[0] == 42.0 && remote.state.phase == 3)
    });
    assert!(seen, "host never saw the guest's position");
    let remote = host.view().remote.expect("remote was just seen");
    assert_eq!(remote.name, "Guesty");
    assert_eq!(remote.slot, 1);
}

#[test]
fn newer_snapshots_replace_older_ones() {
    let network = LoopbackNetwork::new();
    let host = spawn(&network, "Hosty", BUILD);
    let guest = spawn(&network, "Guesty", BUILD);
    connect(&host, &guest);
    let moved = wait_until(|| {
        host.publish(snapshot_at(1.0));
        guest
            .view()
            .remote
            .is_some_and(|remote| remote.state.position[0] == 1.0)
    });
    assert!(moved);
    let moved_again = wait_until(|| {
        host.publish(snapshot_at(2.0));
        guest
            .view()
            .remote
            .is_some_and(|remote| remote.state.position[0] == 2.0)
    });
    assert!(moved_again);
}

#[test]
fn guest_leaving_returns_the_host_to_waiting() {
    let network = LoopbackNetwork::new();
    let host = spawn(&network, "Hosty", BUILD);
    let guest = spawn(&network, "Guesty", BUILD);
    connect(&host, &guest);
    guest.send_command(NetCommand::Leave);
    assert!(wait_until(|| host.view().state == NetState::Hosting));
    assert_eq!(host.view().remote, None);
    assert!(!host.wants_snapshots());
    assert_eq!(guest.view().state, NetState::Idle);
}

#[test]
fn build_mismatch_is_reported_to_the_joiner() {
    let network = LoopbackNetwork::new();
    let host = spawn(&network, "Hosty", BUILD);
    let guest = spawn(&network, "Guesty", OTHER_BUILD);
    host.send_command(NetCommand::Host);
    assert!(wait_until(|| host.view().ticket.is_some()));
    let ticket = host.view().ticket.expect("ticket was just seen");
    guest.send_command(NetCommand::Join(ticket));
    assert!(wait_until(|| {
        guest.view().state == NetState::Rejected(RejectReason::BuildMismatch)
    }));
    assert_eq!(host.view().state, NetState::Hosting);
}

#[test]
fn a_ticket_that_is_not_one_of_ours_fails_without_crashing() {
    let network = LoopbackNetwork::new();
    let guest = spawn(&network, "Guesty", BUILD);
    guest.send_command(NetCommand::Join("not-a-ticket".to_owned()));
    assert!(wait_until(|| matches!(
        guest.view().state,
        NetState::Failed(_)
    )));
}

#[test]
fn two_net_threads_connect_over_real_iroh_on_this_machine() {
    let spawn_iroh = |name: &str| {
        NetThread::spawn(
            || {
                IrohTransport::new(IrohConfig {
                    use_relays_and_lookup: false,
                })
            },
            name.to_owned(),
            BUILD,
        )
        .expect("net thread starts")
    };
    let host = spawn_iroh("Hosty");
    let guest = spawn_iroh("Guesty");
    connect(&host, &guest);
    let seen = wait_until(|| {
        guest.publish(snapshot_at(5.0));
        host.view()
            .remote
            .is_some_and(|remote| remote.state.position[0] == 5.0)
    });
    assert!(seen, "host never saw the guest over iroh");
    assert!(wait_until(|| host.view().direct_path == Some(true)));
}
