use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use rc_launcher_core::{
    LauncherConfig, LauncherCore, LauncherNet, NetState, PathKind, TransportFactory,
};
use rc_net::{
    Channel, Invite, LINK_INVITE_PREFIX, LINK_LAUNCHER_PEER, LocalLinkTransport, LoopbackNetwork,
    Transport, TransportEvent,
};
use rc_proto::{
    EnemyGone, EnemyGoneReason, EnemyId, GameReadiness, Message, PlayerState, RejectReason, decode,
    encode,
};

const WAIT_LIMIT: Duration = Duration::from_secs(20);

pub struct FakeGame {
    link: LocalLinkTransport,
    pub received: Vec<Message>,
}

impl FakeGame {
    pub fn attach(core: &LauncherCore, config: &LauncherConfig) -> Self {
        let mut link = LocalLinkTransport::game(config.build);
        let invite =
            Invite::new(format!("{LINK_INVITE_PREFIX}{}", core.link_port())).expect("invite");
        link.join(&invite).expect("dials");
        Self {
            link,
            received: Vec::new(),
        }
    }

    pub fn pump(&mut self) {
        let mut events = Vec::new();
        self.link.poll(&mut events);
        for event in events {
            if let TransportEvent::Message { payload, .. } = event
                && let Ok(message) = decode(&payload)
            {
                self.received.push(message);
            }
        }
    }

    pub fn send_state(&mut self, state: &PlayerState) {
        let bytes = encode(&Message::State(*state)).expect("encodes");
        let _ = self
            .link
            .send(LINK_LAUNCHER_PEER, Channel::UnreliableLatest, &bytes);
    }

    pub fn send_readiness(&mut self, readiness: GameReadiness) {
        let bytes = encode(&Message::Readiness(readiness)).expect("encodes");
        let _ = self
            .link
            .send(LINK_LAUNCHER_PEER, Channel::ReliableOrdered, &bytes);
    }

    pub fn saw(&mut self, matcher: impl Fn(&Message) -> bool) -> bool {
        self.pump();
        self.received.iter().any(matcher)
    }
}

pub fn wait_until(mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + WAIT_LIMIT;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        thread::sleep(Duration::from_millis(3));
    }
    false
}

pub fn state_at(tick: u32, x: f32) -> PlayerState {
    PlayerState {
        tick,
        phase: 2,
        position: [x, 0.0, 0.0],
        yaw_radians: 0.0,
        motion: None,
        hp: 10,
        max_hp: 10,
        weapon: None,
        activity: Default::default(),
    }
}

fn loopback_factory(network: &LoopbackNetwork) -> TransportFactory {
    let network = network.clone();
    Arc::new(move || Ok(Box::new(network.endpoint()) as Box<dyn Transport>))
}

struct Rig {
    host: LauncherCore,
    guest: LauncherCore,
    host_game: FakeGame,
    guest_game: FakeGame,
}

fn rig() -> Rig {
    let network = LoopbackNetwork::new();
    let host = LauncherCore::new(LauncherConfig::new("Hosty"), loopback_factory(&network))
        .expect("host core");
    let guest = LauncherCore::new(LauncherConfig::new("Guesty"), loopback_factory(&network))
        .expect("guest core");
    let host_game = FakeGame::attach(&host, &LauncherConfig::new("x"));
    let guest_game = FakeGame::attach(&guest, &LauncherConfig::new("x"));
    Rig {
        host,
        guest,
        host_game,
        guest_game,
    }
}

fn connect(rig: &mut Rig, password: Option<&str>) {
    let ticket = rig.host.host(password).expect("ticket");
    rig.guest.join(&ticket, password).expect("join starts");
}

#[test]
fn hosting_returns_a_ticket_and_reports_it_in_the_status() {
    let rig = rig();
    let ticket = rig.host.host(None).expect("ticket");
    let status = rig.host.status();
    assert!(matches!(&status.state, NetState::Hosting { ticket: t } if *t == ticket));
}

#[test]
fn both_games_get_a_welcome_with_the_other_players_name() {
    let mut rig = rig();
    connect(&mut rig, None);
    assert!(wait_until(|| rig.host_game.saw(|m| matches!(
        m,
        Message::Welcome(w) if w.host_name == "Guesty" && w.slot == 0
    ))));
    assert!(wait_until(|| rig.guest_game.saw(|m| matches!(
        m,
        Message::Welcome(w) if w.host_name == "Hosty" && w.slot == 1
    ))));
    assert!(matches!(
        rig.guest.status().state,
        NetState::Connected { .. }
    ));
    assert!(rig.host.link_status().game_linked);
}

#[test]
fn player_state_crosses_both_links_and_the_session() {
    let mut rig = rig();
    connect(&mut rig, None);
    assert!(wait_until(|| rig
        .host_game
        .saw(|m| matches!(m, Message::Welcome(_)))));
    assert!(wait_until(|| rig
        .guest_game
        .saw(|m| matches!(m, Message::Welcome(_)))));
    let mut tick = 0;
    assert!(wait_until(|| {
        tick += 1;
        rig.host_game.send_state(&state_at(tick, 5.0));
        rig.guest_game
            .saw(|m| matches!(m, Message::State(s) if s.position[0] == 5.0 && s.phase == 2))
    }));
    assert!(wait_until(|| {
        tick += 1;
        rig.guest_game.send_state(&state_at(tick, -3.0));
        rig.host_game
            .saw(|m| matches!(m, Message::State(s) if s.position[0] == -3.0))
    }));
}

#[test]
fn an_enemy_event_from_the_host_game_reaches_the_guest_game() {
    let mut rig = rig();
    connect(&mut rig, None);
    assert!(wait_until(|| rig
        .guest_game
        .saw(|m| matches!(m, Message::Welcome(_)))));
    let gone = Message::EnemyGone(EnemyGone {
        phase: 2,
        id: EnemyId {
            group: 40,
            ordinal: 1,
        },
        reason: EnemyGoneReason::Removed,
    });
    let bytes = encode(&gone).expect("encodes");
    assert!(wait_until(|| {
        let _ = rig
            .host_game
            .link
            .send(LINK_LAUNCHER_PEER, Channel::ReliableOrdered, &bytes);
        rig.guest_game.saw(|m| *m == gone)
    }));
}

#[test]
fn path_info_reaches_the_game() {
    let mut rig = rig();
    connect(&mut rig, None);
    assert!(wait_until(|| rig
        .guest_game
        .saw(|m| matches!(m, Message::PathInfo(info) if info.direct))));
}

#[test]
fn the_right_password_connects() {
    let rig = rig();
    let ticket = rig.host.host(Some("hunter2")).expect("ticket");
    rig.guest.join(&ticket, Some("hunter2")).expect("join");
    assert!(wait_until(|| matches!(
        rig.guest.status().state,
        NetState::Connected { .. }
    )));
}

#[test]
fn a_wrong_password_shows_in_the_status_and_the_game_is_told() {
    let mut rig = rig();
    let ticket = rig.host.host(Some("hunter2")).expect("ticket");
    rig.guest.join(&ticket, Some("wrong")).expect("join");
    assert!(wait_until(|| {
        matches!(rig.guest.status().state, NetState::Failed(ref text) if text.contains("password"))
    }));
    assert!(wait_until(|| rig.guest_game.saw(|m| matches!(
        m,
        Message::Reject(r) if r.reason == RejectReason::WrongPassword
    ))));
    assert!(matches!(rig.host.status().state, NetState::Hosting { .. }));
}

#[test]
fn the_host_stopping_tells_the_guest_game_goodbye() {
    let mut rig = rig();
    connect(&mut rig, None);
    assert!(wait_until(|| rig
        .guest_game
        .saw(|m| matches!(m, Message::Welcome(_)))));
    rig.host.leave();
    assert!(wait_until(|| rig
        .guest_game
        .saw(|m| matches!(m, Message::Bye))));
    assert_eq!(rig.host.status().state, NetState::Idle);
}

#[test]
fn a_game_that_attaches_late_is_welcomed_when_the_peer_is_already_here() {
    let mut rig = rig();
    connect(&mut rig, None);
    assert!(wait_until(|| matches!(
        rig.guest.status().state,
        NetState::Connected { .. }
    )));
    let late = FakeGame::attach(&rig.guest, &LauncherConfig::new("x"));
    drop(std::mem::replace(&mut rig.guest_game, late));
    assert!(wait_until(|| rig.guest_game.saw(
        |m| matches!(m, Message::Welcome(w) if w.host_name == "Hosty")
    )));
}

#[test]
fn a_game_on_another_build_cannot_attach() {
    let network = LoopbackNetwork::new();
    let core =
        LauncherCore::new(LauncherConfig::new("Hosty"), loopback_factory(&network)).expect("core");
    let mut other = LauncherConfig::new("x");
    other.build.size_of_image ^= 1;
    let _game = FakeGame::attach(&core, &other);
    thread::sleep(Duration::from_millis(700));
    assert!(!core.link_status().game_linked);
}

#[test]
fn test_connection_reports_a_good_loopback_link() {
    let rig = rig();
    let ticket = rig.host.host(Some("pw")).expect("ticket");
    let report = rig
        .guest
        .test_connection(&ticket, Some("pw"), 2)
        .expect("probe runs");
    assert!(report.reachable);
    assert_eq!(report.path, PathKind::Direct);
    assert!(report.rtt.is_some());
    assert!(report.verdict.contains("good"));
}

#[test]
fn test_connection_with_the_wrong_password_says_so() {
    let rig = rig();
    let ticket = rig.host.host(Some("pw")).expect("ticket");
    let report = rig
        .guest
        .test_connection(&ticket, Some("nope"), 2)
        .expect("probe runs");
    assert!(report.verdict.contains("password is wrong"));
}

#[test]
fn test_connection_to_nobody_reports_unreachable() {
    let network = LoopbackNetwork::new();
    let core =
        LauncherCore::new(LauncherConfig::new("Guesty"), loopback_factory(&network)).expect("core");
    let report = core
        .test_connection("loopback-999", None, 2)
        .map(|report| report.reachable);
    assert!(matches!(report, Ok(false) | Err(_)));
}

#[test]
fn a_host_game_that_is_still_loading_is_reported_to_the_guest() {
    let mut rig = rig();
    connect(&mut rig, None);
    assert!(wait_until(|| rig
        .guest_game
        .saw(|m| *m == Message::Readiness(GameReadiness::Loading))));
    assert_eq!(
        rig.guest.link_status().peer_readiness,
        GameReadiness::Loading
    );
}

#[test]
fn a_host_game_going_in_play_is_reported_to_the_guest_and_its_game() {
    let mut rig = rig();
    connect(&mut rig, None);
    assert!(wait_until(|| {
        rig.host_game.send_readiness(GameReadiness::InPlay);
        rig.guest_game
            .saw(|m| *m == Message::Readiness(GameReadiness::InPlay))
    }));
    assert_eq!(
        rig.guest.link_status().peer_readiness,
        GameReadiness::InPlay
    );
}

#[test]
fn a_host_game_that_closes_is_reported_not_linked_and_a_new_one_starts_loading() {
    let mut rig = rig();
    connect(&mut rig, None);
    assert!(wait_until(|| {
        rig.host_game.send_readiness(GameReadiness::InPlay);
        rig.guest.link_status().peer_readiness == GameReadiness::InPlay
    }));
    let Rig {
        host,
        guest,
        host_game,
        ..
    } = rig;
    drop(host_game);
    assert!(wait_until(
        || guest.link_status().peer_readiness == GameReadiness::NotLinked
    ));
    let _returned = FakeGame::attach(&host, &LauncherConfig::new("x"));
    assert!(wait_until(
        || guest.link_status().peer_readiness == GameReadiness::Loading
    ));
}

#[test]
fn a_game_that_reports_before_a_guest_joins_is_still_known_when_the_guest_arrives() {
    let mut rig = rig();
    let ticket = rig.host.host(None).expect("ticket");
    assert!(wait_until(|| rig.host.link_status().game_linked));
    for _ in 0..15 {
        rig.host_game.send_readiness(GameReadiness::InPlay);
        thread::sleep(Duration::from_millis(20));
    }
    rig.guest.join(&ticket, None).expect("join");
    assert!(wait_until(
        || rig.guest.link_status().peer_readiness == GameReadiness::InPlay
    ));
}

#[test]
fn a_garbage_ticket_is_refused() {
    let rig = rig();
    assert!(rig.guest.join("not a ticket", None).is_err());
}
