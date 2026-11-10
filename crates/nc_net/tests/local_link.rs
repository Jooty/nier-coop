use std::thread;
use std::time::{Duration, Instant};

use nc_net::{
    Channel, DisconnectReason, LINK_GAME_PEER, LINK_LAUNCHER_PEER, LocalLinkTransport, Transport,
    TransportEvent,
};
use replicant_proto::BuildGate;

const BUILD: BuildGate = BuildGate {
    time_date_stamp: 0x60B7_3C00,
    size_of_image: 0x0533_9000,
};
const OTHER_BUILD: BuildGate = BuildGate {
    time_date_stamp: 1,
    size_of_image: 2,
};
const WAIT_LIMIT: Duration = Duration::from_secs(10);

fn wait_for(
    transport: &mut LocalLinkTransport,
    seen: &mut Vec<TransportEvent>,
    done: impl Fn(&[TransportEvent]) -> bool,
) -> bool {
    let deadline = Instant::now() + WAIT_LIMIT;
    while Instant::now() < deadline {
        transport.poll(seen);
        if done(seen) {
            return true;
        }
        thread::sleep(Duration::from_millis(5));
    }
    false
}

fn connected(events: &[TransportEvent]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, TransportEvent::PeerConnected(_)))
}

fn received(events: &[TransportEvent], expected: &[u8]) -> bool {
    events.iter().any(
        |event| matches!(event, TransportEvent::Message { payload, .. } if payload == expected),
    )
}

fn launcher() -> (LocalLinkTransport, nc_net::Invite) {
    let mut launcher = LocalLinkTransport::launcher(Some(BUILD));
    let invite = launcher.host().expect("listens");
    (launcher, invite)
}

fn game(invite: &nc_net::Invite, build: BuildGate) -> LocalLinkTransport {
    let mut game = LocalLinkTransport::game(build);
    game.join(invite).expect("dials");
    game
}

#[test]
fn the_game_and_launcher_connect_and_exchange_frames_both_ways() {
    let (mut launcher, invite) = launcher();
    let mut game = game(&invite, BUILD);
    let (mut launcher_events, mut game_events) = (Vec::new(), Vec::new());
    assert!(wait_for(&mut launcher, &mut launcher_events, connected));
    assert!(wait_for(&mut game, &mut game_events, connected));
    assert_eq!(launcher.client_build(), Some(BUILD));
    game.send(LINK_LAUNCHER_PEER, Channel::UnreliableLatest, b"from game")
        .expect("sends");
    launcher
        .send(LINK_GAME_PEER, Channel::ReliableOrdered, b"from launcher")
        .expect("sends");
    assert!(wait_for(&mut launcher, &mut launcher_events, |e| received(
        e,
        b"from game"
    )));
    assert!(wait_for(&mut game, &mut game_events, |e| received(
        e,
        b"from launcher"
    )));
}

#[test]
fn frames_keep_their_channel() {
    let (mut launcher, invite) = launcher();
    let mut game = game(&invite, BUILD);
    let (mut launcher_events, mut game_events) = (Vec::new(), Vec::new());
    assert!(wait_for(&mut launcher, &mut launcher_events, connected));
    assert!(wait_for(&mut game, &mut game_events, connected));
    game.send(LINK_LAUNCHER_PEER, Channel::UnreliableLatest, b"x")
        .expect("sends");
    assert!(wait_for(&mut launcher, &mut launcher_events, |events| {
        events.iter().any(|event| {
            matches!(
                event,
                TransportEvent::Message {
                    channel: Channel::UnreliableLatest,
                    ..
                }
            )
        })
    }));
}

#[test]
fn a_game_on_another_build_is_refused() {
    let (mut launcher, invite) = launcher();
    let mut game = game(&invite, OTHER_BUILD);
    let mut events = Vec::new();
    assert!(!wait_for_for(
        &mut launcher,
        &mut events,
        Duration::from_millis(800)
    ));
    let mut game_events = Vec::new();
    game.poll(&mut game_events);
    assert!(!connected(&game_events));
}

fn wait_for_for(
    transport: &mut LocalLinkTransport,
    seen: &mut Vec<TransportEvent>,
    limit: Duration,
) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        transport.poll(seen);
        if connected(seen) {
            return true;
        }
        thread::sleep(Duration::from_millis(5));
    }
    false
}

#[test]
fn a_second_game_is_refused_while_one_is_attached() {
    let (mut launcher, invite) = launcher();
    let mut first = game(&invite, BUILD);
    let (mut launcher_events, mut first_events) = (Vec::new(), Vec::new());
    assert!(wait_for(&mut launcher, &mut launcher_events, connected));
    assert!(wait_for(&mut first, &mut first_events, connected));
    let mut second = game(&invite, BUILD);
    let mut second_events = Vec::new();
    assert!(!wait_for_for(
        &mut second,
        &mut second_events,
        Duration::from_millis(800)
    ));
    assert_eq!(
        launcher_events
            .iter()
            .filter(|event| matches!(event, TransportEvent::PeerConnected(_)))
            .count(),
        1
    );
}

#[test]
fn the_game_reconnects_after_the_launcher_side_drops_it() {
    let (mut launcher, invite) = launcher();
    let mut game = game(&invite, BUILD);
    let (mut launcher_events, mut game_events) = (Vec::new(), Vec::new());
    assert!(wait_for(&mut launcher, &mut launcher_events, connected));
    assert!(wait_for(&mut game, &mut game_events, connected));
    launcher.disconnect(LINK_GAME_PEER);
    assert!(wait_for(&mut game, &mut game_events, |events| {
        events.iter().any(|event| {
            matches!(
                event,
                TransportEvent::PeerDisconnected(_, DisconnectReason::Closed)
            )
        })
    }));
    launcher_events.clear();
    assert!(wait_for(&mut launcher, &mut launcher_events, connected));
    launcher
        .send(LINK_GAME_PEER, Channel::ReliableOrdered, b"again")
        .expect("sends");
    assert!(wait_for(&mut game, &mut game_events, |e| received(
        e, b"again"
    )));
}

#[test]
fn sending_with_no_game_attached_fails() {
    let (mut launcher, _invite) = launcher();
    assert!(
        launcher
            .send(LINK_GAME_PEER, Channel::ReliableOrdered, b"x")
            .is_err()
    );
}

#[test]
fn the_launcher_reports_its_port_in_the_invite() {
    let (launcher, invite) = launcher();
    let port = launcher.local_port().expect("bound");
    assert_eq!(invite.as_str(), format!("link-{port}"));
}
