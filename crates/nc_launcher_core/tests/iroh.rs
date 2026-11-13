use std::sync::Arc;

use nc_launcher_core::{
    LauncherConfig, LauncherCore, LauncherEvent, LauncherNet, NetState, PathKind, iroh_factory,
};
use nc_net::{IrohConfig, LinkProgress};

use crate::bridge::{FakeGame, state_at, wait_until};

fn local_core(name: &str) -> LauncherCore {
    let factory = iroh_factory(IrohConfig {
        use_relays_and_lookup: false,
    });
    LauncherCore::new(LauncherConfig::new(name), Arc::clone(&factory)).expect("core")
}

#[test]
fn two_launchers_connect_over_local_iroh_and_relay_state() {
    let host = local_core("Hosty");
    let guest = local_core("Guesty");
    let mut host_game = FakeGame::attach(&host, &LauncherConfig::new("x"));
    let mut guest_game = FakeGame::attach(&guest, &LauncherConfig::new("x"));
    let ticket = host.host().expect("ticket");
    guest.join(&ticket).expect("join");
    assert!(wait_until(|| matches!(
        guest.status().state,
        NetState::Connected { .. }
    )));
    let mut tick = 0;
    assert!(wait_until(|| {
        tick += 1;
        host_game.send_state(&state_at(tick, 9.0));
        guest_game.saw(|m| matches!(m, replicant_proto::Message::State(s) if s.position[0] == 9.0))
    }));
}

#[test]
fn a_connection_test_over_local_iroh_finds_a_direct_path() {
    let host = local_core("Hosty");
    let guest = local_core("Guesty");
    let ticket = host.host().expect("ticket");
    let report = guest.test_connection(&ticket, 4).expect("probe runs");
    assert!(report.reachable);
    assert!(report.rtt.is_some());
    assert_eq!(report.path, PathKind::Direct);
}

#[test]
fn a_join_over_local_iroh_tells_its_progress_and_logs_it() {
    let host = local_core("Hosty");
    let guest = local_core("Guesty");
    let ticket = host.host().expect("ticket");
    assert!(
        host.progress()
            .iter()
            .any(|entry| matches!(entry.progress, LinkProgress::OwnId(_)))
    );
    guest.join(&ticket).expect("join");
    assert!(wait_until(|| matches!(
        guest.status().state,
        NetState::Connected { .. }
    )));
    let told: Vec<LinkProgress> = guest.progress().into_iter().map(|e| e.progress).collect();
    for wanted in [
        |p: &LinkProgress| matches!(p, LinkProgress::TicketRead { .. }),
        |p: &LinkProgress| matches!(p, LinkProgress::HostAnswered { .. }),
        |p: &LinkProgress| matches!(p, LinkProgress::HelloSent { .. }),
        |p: &LinkProgress| matches!(p, LinkProgress::WelcomeRead { seat: 1, .. }),
    ] {
        assert!(told.iter().any(wanted), "{told:?}");
    }
    let mut logged = Vec::new();
    assert!(wait_until(|| {
        logged.extend(guest.drain_events());
        logged.iter().any(|event| {
            matches!(event, LauncherEvent::LinkProgress { line, .. } if line.starts_with("welcome read from Hosty"))
        })
    }));
}
