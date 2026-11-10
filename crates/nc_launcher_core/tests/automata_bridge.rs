use std::sync::Arc;
use std::thread;
use std::time::Duration;

use automata_game::build::{COMPAT_PE_IDENTITY, MAIN_PE_IDENTITY};
use automata_game::character::Character;
use automata_game::exe::ExeVariant;
use automata_proto::{CharacterHello, Message, PROTOCOL_VERSION, PlayerState, decode, encode};
use nc_launcher_core::{
    AUTOMATA, LauncherConfig, LauncherCore, LauncherEvent, LauncherNet, NetState, REPLICANT,
    TransportFactory, version_difference_text,
};
use nc_net::{
    Channel, Invite, LINK_INVITE_PREFIX, LINK_LAUNCHER_PEER, LocalLinkTransport, LoopbackNetwork,
    Transport, TransportEvent,
};
use nc_wire::{BuildGate, GameReadiness, Hello, LinkPayload, Reject, RejectReason, Welcome};
use replicant_game::build::PeIdentity;

use crate::bridge::wait_until;

const MAIN_BUILD: BuildGate = build_gate_of(MAIN_PE_IDENTITY);
const COMPAT_BUILD: BuildGate = build_gate_of(COMPAT_PE_IDENTITY);

struct FakeHook {
    link: LocalLinkTransport,
    received: Vec<Message>,
}

struct Rig {
    host: LauncherCore,
    guest: LauncherCore,
    host_hook: FakeHook,
    guest_hook: FakeHook,
}

impl FakeHook {
    fn attach(core: &LauncherCore, build: BuildGate) -> Self {
        let mut link = LocalLinkTransport::game(build);
        let invite =
            Invite::new(format!("{LINK_INVITE_PREFIX}{}", core.link_port())).expect("invite");
        link.join(&invite).expect("dials");
        Self {
            link,
            received: Vec::new(),
        }
    }

    fn send(&mut self, message: &Message) {
        let bytes = encode(message).expect("encodes");
        let channel = if message.is_latest_wins() {
            Channel::UnreliableLatest
        } else {
            Channel::ReliableOrdered
        };
        let _ = self.link.send(LINK_LAUNCHER_PEER, channel, &bytes);
    }

    fn saw(&mut self, matcher: impl Fn(&Message) -> bool) -> bool {
        let mut events = Vec::new();
        self.link.poll(&mut events);
        for event in events {
            if let TransportEvent::Message { payload, .. } = event {
                self.received
                    .push(decode(&payload).expect("the launcher sends this game's messages"));
            }
        }
        self.received.iter().any(matcher)
    }
}

const fn build_gate_of(identity: PeIdentity) -> BuildGate {
    BuildGate {
        time_date_stamp: identity.time_date_stamp,
        size_of_image: identity.size_of_image,
    }
}

fn loopback_factory(network: &LoopbackNetwork) -> TransportFactory {
    let network = network.clone();
    Arc::new(move || Ok(Box::new(network.endpoint()) as Box<dyn Transport>))
}

fn automata_core(network: &LoopbackNetwork, name: &str) -> LauncherCore {
    LauncherCore::for_game(&AUTOMATA, name, loopback_factory(network)).expect("core")
}

fn replicant_core(network: &LoopbackNetwork, name: &str) -> LauncherCore {
    LauncherCore::new(LauncherConfig::new(name), loopback_factory(network)).expect("core")
}

fn connected_rig() -> Rig {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    let guest = automata_core(&network, "Guesty");
    let host_hook = FakeHook::attach(&host, MAIN_BUILD);
    let guest_hook = FakeHook::attach(&guest, COMPAT_BUILD);
    let ticket = host.host().expect("ticket");
    guest.join(&ticket).expect("join starts");
    let mut rig = Rig {
        host,
        guest,
        host_hook,
        guest_hook,
    };
    assert!(wait_until(|| rig.host_hook.saw(|m| matches!(
        m,
        Message::Welcome(w) if w.host_name == "Guesty" && w.slot == 0
    ))));
    assert!(wait_until(|| rig.guest_hook.saw(|m| matches!(
        m,
        Message::Welcome(w) if w.host_name == "Hosty" && w.slot == 1
    ))));
    rig
}

fn state_at(sequence: u32, x: f32) -> PlayerState {
    PlayerState {
        sequence,
        held: 1,
        pressed: 1,
        speed: 1.0,
        world_facing: 0.5,
        position: [x, 0.0, 0.0],
        body_facing: 0.25,
        state: 3,
        last_start: None,
        health: 100,
        free: true,
        phase_id: 0x100,
    }
}

fn saw_event(core: &LauncherCore, wanted: impl Fn(&LauncherEvent) -> bool) -> bool {
    wait_until(|| core.drain_events().iter().any(&wanted))
}

fn failure_text(core: &LauncherCore) -> Option<String> {
    match core.status().state {
        NetState::Failed(text) => Some(text),
        _ => None,
    }
}

#[test]
fn both_hooks_are_welcomed_with_this_games_protocol_version() {
    let mut rig = connected_rig();
    assert!(rig.host_hook.saw(|m| matches!(
        m,
        Message::Welcome(w) if w.version == PROTOCOL_VERSION
    )));
    assert!(rig.guest_hook.saw(|m| matches!(
        m,
        Message::Welcome(w) if w.version == PROTOCOL_VERSION
    )));
    assert!(wait_until(|| rig.host.link_status().game_linked));
}

#[test]
fn player_state_crosses_both_ways_between_the_hooks() {
    let mut rig = connected_rig();
    assert!(wait_until(|| {
        rig.host_hook.send(&Message::State(state_at(5, 11.0)));
        rig.guest_hook
            .saw(|m| matches!(m, Message::State(s) if s.position[0] == 11.0))
    }));
    assert!(wait_until(|| {
        rig.guest_hook.send(&Message::State(state_at(9, 22.0)));
        rig.host_hook
            .saw(|m| matches!(m, Message::State(s) if s.position[0] == 22.0))
    }));
}

#[test]
fn the_character_hello_crosses_both_ways_between_the_hooks() {
    let mut rig = connected_rig();
    rig.host_hook.send(&Message::Character(CharacterHello {
        character: Character::TwoB,
        exe: ExeVariant::Main,
    }));
    rig.guest_hook.send(&Message::Character(CharacterHello {
        character: Character::A2,
        exe: ExeVariant::Main,
    }));
    assert!(wait_until(|| rig.guest_hook.saw(|m| matches!(
        m,
        Message::Character(hello) if hello.character == Character::TwoB
    ))));
    assert!(wait_until(|| rig.host_hook.saw(|m| matches!(
        m,
        Message::Character(hello) if hello.character == Character::A2
    ))));
}

#[test]
fn the_ping_of_the_session_reaches_both_hooks() {
    let mut rig = connected_rig();
    assert!(wait_until(|| rig
        .host_hook
        .saw(|m| matches!(m, Message::PathInfo(info) if info.direct))));
    assert!(wait_until(|| rig
        .guest_hook
        .saw(|m| matches!(m, Message::PathInfo(info) if info.direct))));
}

#[test]
fn the_lobby_character_pick_reaches_the_other_launcher_and_no_hook() {
    let mut rig = connected_rig();
    rig.host.send_lobby_character(0);
    rig.guest.send_lobby_character(2);
    assert!(wait_until(
        || rig.host.link_status().peer_character == Some(2)
    ));
    assert!(wait_until(
        || rig.guest.link_status().peer_character == Some(0)
    ));
    thread::sleep(Duration::from_millis(200));
    assert!(
        !rig.host_hook
            .saw(|m| matches!(m, Message::LobbyCharacter(_)))
    );
    assert!(
        !rig.guest_hook
            .saw(|m| matches!(m, Message::LobbyCharacter(_)))
    );
}

#[test]
fn a_character_set_before_joining_is_sent_when_the_guest_arrives() {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    let guest = automata_core(&network, "Guesty");
    let ticket = host.host().expect("ticket");
    guest.send_lobby_character(2);
    guest.join(&ticket).expect("join starts");
    assert!(wait_until(|| host.link_status().peer_character == Some(2)));
}

#[test]
fn the_guest_launcher_hears_that_the_hosts_game_is_linked() {
    let rig = connected_rig();
    assert!(wait_until(
        || rig.guest.link_status().peer_readiness == GameReadiness::Loading
    ));
}

#[test]
fn the_lobby_start_reaches_the_guests_launcher() {
    let rig = connected_rig();
    assert!(wait_until(|| {
        rig.host.start_lobby(0);
        rig.guest
            .drain_events()
            .iter()
            .any(|event| matches!(event, LauncherEvent::LobbyStartReceived { mode: 0 }))
    }));
}

#[test]
fn a_hook_of_the_other_game_cannot_attach() {
    let network = LoopbackNetwork::new();
    let core = automata_core(&network, "Hosty");
    let _hook = FakeHook::attach(&core, LauncherConfig::new("x").build);
    thread::sleep(Duration::from_millis(700));
    assert!(!core.link_status().game_linked);
}

#[test]
fn a_launcher_changed_to_this_game_takes_its_hook_and_talks_to_its_launchers() {
    let network = LoopbackNetwork::new();
    let host = replicant_core(&network, "Hosty");
    host.set_game(&AUTOMATA);
    let guest = automata_core(&network, "Guesty");
    let mut host_hook = FakeHook::attach(&host, COMPAT_BUILD);
    let ticket = host.host().expect("ticket");
    guest.join(&ticket).expect("join starts");
    assert!(wait_until(|| host_hook.saw(|m| matches!(
        m,
        Message::Welcome(w) if w.host_name == "Guesty" && w.version == PROTOCOL_VERSION
    ))));
}

#[test]
fn a_character_picked_under_this_game_is_never_sent_under_the_other_game() {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    host.send_lobby_character(1);
    host.set_game(&REPLICANT);
    let guest = replicant_core(&network, "Guesty");
    let ticket = host.host().expect("ticket");
    guest.join(&ticket).expect("join starts");
    assert!(wait_until(
        || guest.link_status().peer_name.as_deref() == Some("Hosty")
    ));
    host.send_lobby_character(2);
    thread::sleep(Duration::from_millis(300));
    assert_eq!(guest.link_status().peer_character, None);
    assert_eq!(failure_text(&guest), None);
}

#[test]
fn a_host_on_this_game_refuses_a_guest_on_the_other_game_and_says_why() {
    let network = LoopbackNetwork::new();
    let host = automata_core(&network, "Hosty");
    let guest = replicant_core(&network, "Guesty");
    let ticket = host.host().expect("ticket");
    guest.join(&ticket).expect("join starts");
    assert!(saw_event(&host, |event| matches!(
        event,
        LauncherEvent::PeerOnOtherVersion {
            ours: 1007,
            theirs: 35
        }
    )));
    assert!(wait_until(|| failure_text(&guest).is_some()));
    let text = failure_text(&guest).expect("the guest shows why");
    assert!(text.contains("another game open"), "{text}");
}

#[test]
fn a_host_on_the_other_game_refuses_a_guest_on_this_game_and_says_why() {
    let network = LoopbackNetwork::new();
    let host = replicant_core(&network, "Hosty");
    let guest = automata_core(&network, "Guesty");
    let ticket = host.host().expect("ticket");
    guest.join(&ticket).expect("join starts");
    assert!(saw_event(&host, |event| matches!(
        event,
        LauncherEvent::PeerOnOtherVersion {
            ours: 35,
            theirs: 1007
        }
    )));
    assert!(wait_until(|| failure_text(&guest).is_some()));
    let text = failure_text(&guest).expect("the guest shows why");
    assert!(text.contains("another game open"), "{text}");
}

#[test]
fn the_refusal_line_names_both_games_or_both_versions() {
    let games = version_difference_text(35, 1007);
    assert!(games.contains(AUTOMATA.display_name), "{games}");
    assert!(games.contains(REPLICANT.display_name), "{games}");
    let releases = version_difference_text(35, 34);
    assert!(releases.contains("protocol 34"), "{releases}");
    assert!(releases.contains("35"), "{releases}");
}

#[test]
fn the_handshake_bytes_are_the_same_under_both_games() {
    let payloads = [
        LinkPayload::Hello(Hello {
            version: 35,
            build: MAIN_BUILD,
            name: "Nier".to_owned(),
        }),
        LinkPayload::Welcome(Welcome {
            version: 1007,
            slot: 1,
            host_name: "Host".to_owned(),
        }),
        LinkPayload::Reject(Reject {
            reason: RejectReason::VersionMismatch,
        }),
    ];
    for payload in payloads {
        let this_game: Message = nc_wire::GameMessage::from_link(payload.clone());
        let other_game: replicant_proto::Message = nc_wire::GameMessage::from_link(payload);
        assert_eq!(
            encode(&this_game).expect("encodes"),
            replicant_proto::encode(&other_game).expect("encodes")
        );
    }
}
