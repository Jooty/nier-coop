use std::thread;
use std::time::{Duration, Instant};

use automata_game::character::Character;
use automata_game::exe::ExeVariant;
use automata_game::partner::BodySnapshot;
use automata_hook::net::{
    AutomataNet, NetShared, ReadinessPacer, co_op_status_of, link_not_started_status, readiness_of,
};
use automata_proto::{
    CharacterHello, GameReadiness, Message, PROTOCOL_VERSION, PathInfo, PlayerState,
    TeleportRequest, Welcome,
};
use nc_hook::net::{LinkPeer, NetState, NetThread, RemotePlayer};
use nc_net::{LoopbackNetwork, Transport};

const WAIT_LIMIT: Duration = Duration::from_secs(20);

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

fn welcome(slot: u8, host_name: &str) -> Message {
    Message::Welcome(Welcome {
        version: PROTOCOL_VERSION,
        slot,
        host_name: host_name.to_owned(),
    })
}

fn connected_pair() -> (NetThread<AutomataNet>, NetThread<AutomataNet>) {
    let network = LoopbackNetwork::new();
    let a_transport = network.endpoint();
    let mut b_transport = network.endpoint();
    let a_peer = a_transport.local_peer();
    let b_peer = b_transport.local_peer();
    let b_invite = b_transport.host().expect("b hosts");
    let a = NetThread::spawn(
        AutomataNet::new(Character::NineS, ExeVariant::Main),
        a_transport,
        LinkPeer {
            invite: Some(b_invite),
            peer: b_peer,
            port: 1,
        },
    )
    .expect("a starts");
    let b = NetThread::spawn(
        AutomataNet::new(Character::TwoB, ExeVariant::Compat),
        b_transport,
        LinkPeer {
            invite: None,
            peer: a_peer,
            port: 2,
        },
    )
    .expect("b starts");
    assert!(wait_until(|| {
        a.view().state == NetState::LauncherLinked && b.view().state == NetState::LauncherLinked
    }));
    a.send_to_launcher(welcome(1, "Mira"));
    b.send_to_launcher(welcome(0, "Odo"));
    assert!(wait_until(|| {
        a.view().state == NetState::Connected && b.view().state == NetState::Connected
    }));
    (a, b)
}

fn snapshot() -> BodySnapshot {
    BodySnapshot {
        held: 0b100,
        pressed: 0b001,
        speed: 1.0,
        world_facing: 1.5,
        position: [10.0, 2.0, -30.0],
        body_facing: 0.5,
        state: 0x44,
        last_start: None,
        health: 4200,
    }
}

#[test]
fn a_published_snapshot_arrives_as_the_partners_state_and_input() {
    let (a, b) = connected_pair();
    assert!(a.wants_snapshots());
    a.publish(snapshot());
    assert!(wait_until(|| b.view().remote.is_some()));
    let remote = b.view().remote.expect("the state arrived");
    assert_eq!(remote.state.to_snapshot(), snapshot());
    assert_eq!(remote.name, "Mira");
    let feed = b.update(|shared| shared.extras.remote_input.take_feed());
    assert_eq!(feed.held, 0b100);
    assert_eq!(feed.pressed, 0b101);
    assert_eq!(
        b.update(|shared| shared.extras.remote_input.take_feed().pressed),
        0
    );
}

#[test]
fn the_character_message_reaches_the_other_side_reliably() {
    let (a, b) = connected_pair();
    a.send_relayed(Message::Character(CharacterHello {
        character: Character::A2,
        exe: ExeVariant::Main,
    }));
    assert!(wait_until(|| {
        b.view().extras.partner_character == Some(Character::A2)
    }));
    assert_eq!(a.view().extras.partner_character, None);
}

#[test]
fn the_other_game_counts_as_heard_after_its_hello_or_its_first_state() {
    let (a, b) = connected_pair();
    assert!(!a.view().extras.partner_heard);
    assert!(!b.view().extras.partner_heard);
    a.send_relayed(Message::Character(CharacterHello {
        character: Character::A2,
        exe: ExeVariant::Main,
    }));
    assert!(wait_until(|| b.view().extras.partner_heard));
    b.publish(snapshot());
    assert!(wait_until(|| a.view().extras.partner_heard));
}

#[test]
fn a_teleport_request_waits_in_the_inbox_for_the_game_thread() {
    let (a, b) = connected_pair();
    let request = Message::Teleport(TeleportRequest::FollowJump { point: 8 });
    a.send_relayed(request.clone());
    let mut taken = Vec::new();
    assert!(wait_until(|| {
        taken = b.take_relayed(|message| matches!(message, Message::Teleport(_)));
        !taken.is_empty()
    }));
    assert_eq!(taken, vec![request]);
    assert!(
        a.take_relayed(|message| matches!(message, Message::Teleport(_)))
            .is_empty()
    );
}

#[test]
fn the_path_report_sets_the_ping() {
    let (a, b) = connected_pair();
    a.send_to_launcher(Message::PathInfo(PathInfo {
        rtt_micros: 42_000,
        direct: true,
    }));
    assert!(wait_until(|| b.view().rtt.is_some()));
    let status = co_op_status_of(&b.view(), Character::TwoB);
    assert_eq!(status.ping_ms, Some(42));
    assert_eq!(status.link, "connected");
}

fn shared(state: NetState) -> NetShared {
    NetShared {
        state,
        peer_name: Some("Mira".to_owned()),
        ..NetShared::default()
    }
}

#[test]
fn the_co_op_section_is_empty_until_a_partner_is_connected() {
    let status = co_op_status_of(&shared(NetState::LauncherLinked), Character::NineS);
    assert_eq!(status.link, "launcher linked, no other player");
    assert!(status.players.is_empty());
    assert_eq!(status.ping_ms, None);
    let off = link_not_started_status();
    assert!(off.link.starts_with("not started"));
    assert!(off.players.is_empty());
}

#[test]
fn a_connected_partner_shows_the_link_setting_until_it_names_its_character() {
    let mut connected = shared(NetState::Connected);
    let status = co_op_status_of(&connected, Character::NineS);
    assert_eq!(status.players.len(), 1);
    assert_eq!(status.players[0].name, "Mira");
    assert!(status.players[0].detail.starts_with("9S (the link setting"));
    connected.extras.partner_character = Some(Character::A2);
    connected.remote = Some(RemotePlayer {
        name: "Mira".to_owned(),
        slot: 1,
        state: PlayerState::from_snapshot(&snapshot(), 1),
        received_at: Instant::now(),
    });
    let status = co_op_status_of(&connected, Character::NineS);
    assert_eq!(status.players[0].detail, "A2, health 4200");
}

#[test]
fn the_launcher_hears_in_play_only_once_the_start_reached_the_world() {
    assert_eq!(readiness_of(false), GameReadiness::Loading);
    assert_eq!(readiness_of(true), GameReadiness::InPlay);
}

#[test]
fn readiness_is_told_at_once_on_a_change_and_again_every_second() {
    let mut pacer = ReadinessPacer::default();
    assert!(pacer.is_due(0, GameReadiness::Loading));
    assert!(pacer.is_new(GameReadiness::Loading));
    pacer.mark_sent(0, GameReadiness::Loading);
    assert!(!pacer.is_due(500, GameReadiness::Loading));
    assert!(pacer.is_due(1_000, GameReadiness::Loading));
    assert!(!pacer.is_new(GameReadiness::Loading));
    assert!(pacer.is_due(10, GameReadiness::InPlay));
    assert!(pacer.is_new(GameReadiness::InPlay));
    pacer.mark_sent(10, GameReadiness::InPlay);
    assert!(pacer.is_due(20, GameReadiness::Loading));
    pacer.forget();
    assert!(pacer.is_due(30, GameReadiness::InPlay));
    assert!(pacer.is_new(GameReadiness::InPlay));
}
