use std::thread;
use std::time::{Duration, Instant};

use rc_hook::net::{LocalSnapshot, NetState, NetThread};
use rc_net::{Channel, LINK_GAME_PEER, LocalLinkTransport, Transport, TransportEvent};
use rc_proto::{
    BuildGate, Message, PathInfo, PlayerState, Reject, RejectReason, Welcome, decode, encode,
};

const BUILD: BuildGate = BuildGate {
    time_date_stamp: 0x60B7_3C00,
    size_of_image: 0x0533_9000,
};
const WAIT_LIMIT: Duration = Duration::from_secs(20);

struct FakeLauncher {
    link: LocalLinkTransport,
    received: Vec<Message>,
}

impl FakeLauncher {
    fn start() -> (Self, u16) {
        let mut link = LocalLinkTransport::launcher(Some(BUILD));
        link.host().expect("listens");
        let port = link.local_port().expect("bound");
        (
            Self {
                link,
                received: Vec::new(),
            },
            port,
        )
    }

    fn pump(&mut self) {
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

    fn say(&mut self, message: &Message) {
        let bytes = encode(message).expect("encodes");
        let _ = self
            .link
            .send(LINK_GAME_PEER, Channel::ReliableOrdered, &bytes);
    }

    fn wait_until(&mut self, mut condition: impl FnMut(&mut Self) -> bool) -> bool {
        let deadline = Instant::now() + WAIT_LIMIT;
        while Instant::now() < deadline {
            self.pump();
            if condition(self) {
                return true;
            }
            thread::sleep(Duration::from_millis(2));
        }
        false
    }
}

fn start_game() -> (FakeLauncher, NetThread) {
    let (launcher, port) = FakeLauncher::start();
    let game = NetThread::spawn(LocalLinkTransport::game(BUILD), port).expect("net thread starts");
    (launcher, game)
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

fn remote_state(x: f32) -> PlayerState {
    snapshot_at(x).to_player_state(1)
}

fn welcome(slot: u8, name: &str) -> Message {
    Message::Welcome(Welcome {
        version: rc_proto::PROTOCOL_VERSION,
        slot,
        host_name: name.to_owned(),
    })
}

fn link_up(launcher: &mut FakeLauncher, game: &NetThread) {
    assert!(wait_until(|| game.view().state == NetState::LauncherLinked));
    launcher.say(&welcome(1, "Hosty"));
    assert!(wait_until(|| game.view().state == NetState::Connected));
}

#[test]
fn the_game_waits_for_the_launcher_then_links() {
    let (mut launcher, port) = FakeLauncher::start();
    let game = NetThread::spawn(LocalLinkTransport::game(BUILD), port).expect("starts");
    assert!(wait_until(|| game.view().state == NetState::LauncherLinked));
    assert_eq!(game.view().link_port, Some(port));
    assert!(!game.wants_snapshots());
    launcher.pump();
}

#[test]
fn without_a_listening_launcher_the_state_stays_waiting() {
    let (launcher, port) = FakeLauncher::start();
    drop(launcher);
    let game = NetThread::spawn(LocalLinkTransport::game(BUILD), port).expect("starts");
    thread::sleep(Duration::from_millis(300));
    assert_eq!(game.view().state, NetState::WaitingForLauncher);
}

#[test]
fn a_welcome_from_the_launcher_names_the_peer_and_starts_snapshots() {
    let (mut launcher, game) = start_game();
    link_up(&mut launcher, &game);
    assert_eq!(game.view().peer_name.as_deref(), Some("Hosty"));
    assert!(game.wants_snapshots());
}

#[test]
fn relayed_player_state_reaches_the_store() {
    let (mut launcher, game) = start_game();
    link_up(&mut launcher, &game);
    launcher.say(&Message::State(remote_state(42.0)));
    assert!(wait_until(|| {
        game.view()
            .remote
            .is_some_and(|remote| remote.state.position[0] == 42.0 && remote.state.phase == 3)
    }));
    let remote = game.view().remote.expect("remote was just seen");
    assert_eq!(remote.name, "Hosty");
    assert_eq!(remote.slot, 0);
}

#[test]
fn published_snapshots_reach_the_launcher_as_player_state() {
    let (mut launcher, game) = start_game();
    link_up(&mut launcher, &game);
    let seen = launcher.wait_until(|launcher| {
        game.publish(snapshot_at(7.0));
        launcher.received.iter().any(|message| {
            matches!(message, Message::State(state) if state.position[0] == 7.0 && state.phase == 3)
        })
    });
    assert!(seen, "the launcher never saw the game's position");
}

#[test]
fn path_info_fills_rtt_and_path() {
    let (mut launcher, game) = start_game();
    link_up(&mut launcher, &game);
    launcher.say(&Message::PathInfo(PathInfo {
        rtt_micros: 12_500,
        direct: false,
    }));
    assert!(wait_until(|| game.view().direct_path == Some(false)));
    assert_eq!(game.view().rtt, Some(Duration::from_micros(12_500)));
}

#[test]
fn a_bye_from_the_launcher_clears_the_peer() {
    let (mut launcher, game) = start_game();
    link_up(&mut launcher, &game);
    launcher.say(&Message::State(remote_state(1.0)));
    assert!(wait_until(|| game.view().remote.is_some()));
    launcher.say(&Message::Bye);
    assert!(wait_until(|| game.view().state == NetState::LauncherLinked));
    assert_eq!(game.view().remote, None);
    assert!(!game.wants_snapshots());
}

#[test]
fn a_reject_is_shown() {
    let (mut launcher, game) = start_game();
    assert!(wait_until(|| game.view().state == NetState::LauncherLinked));
    launcher.say(&Message::Reject(Reject {
        reason: RejectReason::WrongPassword,
    }));
    assert!(wait_until(|| {
        game.view().state == NetState::Rejected(RejectReason::WrongPassword)
    }));
}

#[test]
fn losing_the_launcher_returns_to_waiting_and_relinks() {
    let (mut launcher, game) = start_game();
    link_up(&mut launcher, &game);
    launcher.link.disconnect(LINK_GAME_PEER);
    assert!(wait_until(|| {
        game.view().state == NetState::WaitingForLauncher
            || game.view().state == NetState::LauncherLinked
    }));
    assert!(wait_until(|| game.view().state == NetState::LauncherLinked));
    assert_eq!(game.view().remote, None);
}
