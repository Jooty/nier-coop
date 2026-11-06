use std::sync::Arc;

use rc_launcher_core::{
    LauncherConfig, LauncherCore, LauncherNet, NetState, PathKind, iroh_factory,
};
use rc_net::IrohConfig;

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
        guest_game.saw(|m| matches!(m, rc_proto::Message::State(s) if s.position[0] == 9.0))
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
