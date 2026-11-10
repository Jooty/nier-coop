use std::time::Instant;

use nc_net::{
    Channel, LeaveReason, LoopbackNetwork, LoopbackTransport, PeerId, Role, Session, SessionConfig,
    SessionEvent, Transport,
};
use nc_wire::{
    BuildGate, GameMessage, Hello, LinkPayload, Ping, Pong, Reject, RejectReason, Welcome,
    WireError, encode,
};
use serde::{Deserialize, Serialize};

const BUILD: BuildGate = BuildGate {
    time_date_stamp: 7,
    size_of_image: 9,
};
const THREE_PLAYERS: u8 = 3;
const MAX_CHAT_BYTES: usize = 16;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Msg<const VERSION: u16> {
    Hello(Hello),
    Welcome(Welcome),
    Reject(Reject),
    Ping(Ping),
    Pong(Pong),
    Bye,
    Position { tick: u32, x: i32 },
    Chat(String),
    LocalOnly(u8),
}

impl<const VERSION: u16> GameMessage for Msg<VERSION> {
    const PROTOCOL_VERSION: u16 = VERSION;
    const MAX_PLAYERS: u8 = THREE_PLAYERS;

    fn from_link(payload: LinkPayload) -> Self {
        match payload {
            LinkPayload::Hello(hello) => Msg::Hello(hello),
            LinkPayload::Welcome(welcome) => Msg::Welcome(welcome),
            LinkPayload::Reject(reject) => Msg::Reject(reject),
            LinkPayload::Ping(ping) => Msg::Ping(ping),
            LinkPayload::Pong(pong) => Msg::Pong(pong),
            LinkPayload::Bye => Msg::Bye,
        }
    }

    fn into_link(self) -> Result<LinkPayload, Self> {
        match self {
            Msg::Hello(hello) => Ok(LinkPayload::Hello(hello)),
            Msg::Welcome(welcome) => Ok(LinkPayload::Welcome(welcome)),
            Msg::Reject(reject) => Ok(LinkPayload::Reject(reject)),
            Msg::Ping(ping) => Ok(LinkPayload::Ping(ping)),
            Msg::Pong(pong) => Ok(LinkPayload::Pong(pong)),
            Msg::Bye => Ok(LinkPayload::Bye),
            other => Err(other),
        }
    }

    fn is_relayed(&self) -> bool {
        matches!(self, Msg::Chat(_))
    }

    fn is_latest_wins(&self) -> bool {
        matches!(self, Msg::Position { .. })
    }

    fn ordered_tick(&self) -> Option<u32> {
        match self {
            Msg::Position { tick, .. } => Some(*tick),
            _ => None,
        }
    }

    fn validate(&self) -> Result<(), WireError> {
        match self {
            Msg::Hello(hello) if hello.name.is_empty() => {
                Err(WireError::OutOfRange { field: "name" })
            }
            Msg::Chat(text) if text.len() > MAX_CHAT_BYTES => {
                Err(WireError::OutOfRange { field: "chat" })
            }
            _ => Ok(()),
        }
    }
}

type Current = Msg<4>;
type Older = Msg<3>;

fn config(role: Role, name: &str) -> SessionConfig {
    SessionConfig {
        role,
        name: name.to_owned(),
        build: BUILD,
    }
}

fn pump<M: GameMessage>(
    sessions: &mut [(
        &mut Session<LoopbackTransport, M>,
        &mut Vec<SessionEvent<M>>,
    )],
) {
    let now = Instant::now();
    for _ in 0..6 {
        for (session, events) in sessions.iter_mut() {
            session.poll(now, events);
        }
    }
}

fn relayed_chats(events: &[SessionEvent<Current>]) -> Vec<(PeerId, String)> {
    events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::PeerRelayed {
                peer,
                message: Msg::Chat(text),
            } => Some((*peer, text.clone())),
            _ => None,
        })
        .collect()
}

struct Trio {
    network: LoopbackNetwork,
    host: Session<LoopbackTransport, Current>,
    first: Session<LoopbackTransport, Current>,
    second: Session<LoopbackTransport, Current>,
    host_events: Vec<SessionEvent<Current>>,
    first_events: Vec<SessionEvent<Current>>,
    second_events: Vec<SessionEvent<Current>>,
}

impl Trio {
    fn connected() -> Self {
        let network = LoopbackNetwork::new();
        let mut host = Session::new(network.endpoint(), config(Role::Host, "Host"));
        let mut first = Session::new(network.endpoint(), config(Role::Guest, "First"));
        let mut second = Session::new(network.endpoint(), config(Role::Guest, "Second"));
        let invite = host.host().expect("invite");
        first.join(&invite).expect("join");
        second.join(&invite).expect("join");
        let mut trio = Self {
            network,
            host,
            first,
            second,
            host_events: Vec::new(),
            first_events: Vec::new(),
            second_events: Vec::new(),
        };
        trio.pump();
        trio
    }

    fn pump(&mut self) {
        pump(&mut [
            (&mut self.host, &mut self.host_events),
            (&mut self.first, &mut self.first_events),
            (&mut self.second, &mut self.second_events),
        ]);
    }
}

#[test]
fn a_host_and_two_guests_complete_the_handshake_with_their_own_slots() {
    let trio = Trio::connected();
    let slots: Vec<u8> = trio
        .host_events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::PeerJoined { slot, .. } => Some(*slot),
            _ => None,
        })
        .collect();
    assert_eq!(slots, vec![1, 2]);
    assert_eq!(trio.first.local_slot(), Some(1));
    assert_eq!(trio.second.local_slot(), Some(2));
    assert_eq!(trio.host.ready_peers().len(), 2);
}

#[test]
fn a_relayed_kind_reaches_the_host_from_each_guest() {
    let mut trio = Trio::connected();
    trio.first
        .send_relayed(&Msg::Chat("one".to_owned()))
        .expect("relayed kind");
    trio.second
        .send_relayed(&Msg::Chat("two".to_owned()))
        .expect("relayed kind");
    trio.pump();
    let mut texts: Vec<String> = relayed_chats(&trio.host_events)
        .into_iter()
        .map(|(_, text)| text)
        .collect();
    texts.sort();
    assert_eq!(texts, vec!["one".to_owned(), "two".to_owned()]);
    assert!(relayed_chats(&trio.second_events).is_empty());
}

#[test]
fn a_local_kind_is_not_sent_and_not_accepted_from_a_peer() {
    let mut trio = Trio::connected();
    assert!(trio.first.send_relayed(&Msg::LocalOnly(1)).is_err());
    trio.pump();
    assert!(!trio.host_events.iter().any(|event| matches!(
        event,
        SessionEvent::PeerRelayed { .. } | SessionEvent::PeerOrdered { .. }
    )));
}

#[test]
fn a_local_kind_from_a_raw_peer_ends_that_peer_as_a_violation() {
    let network = LoopbackNetwork::new();
    let mut host: Session<LoopbackTransport, Current> =
        Session::new(network.endpoint(), config(Role::Host, "Host"));
    let mut raw = network.endpoint();
    let invite = host.host().expect("invite");
    raw.join(&invite).expect("join");
    let host_id = host.transport().local_peer();
    let hello = Current::Hello(Hello {
        version: 4,
        build: BUILD,
        name: "Raw".to_owned(),
    });
    let mut events = Vec::new();
    let now = Instant::now();
    for message in [hello, Msg::LocalOnly(3)] {
        raw.send(
            host_id,
            Channel::ReliableOrdered,
            &encode(&message).expect("encodes"),
        )
        .expect("sends");
        host.poll(now, &mut events);
    }
    assert!(events.iter().any(|event| matches!(
        event,
        SessionEvent::PeerLeft {
            reason: LeaveReason::ProtocolViolation,
            ..
        }
    )));
}

#[test]
fn ordered_messages_arrive_and_older_ticks_are_dropped() {
    let mut trio = Trio::connected();
    for tick in [5, 4, 6] {
        trio.first
            .send_ordered(&Msg::Position { tick, x: 0 })
            .expect("ordered kind");
    }
    trio.pump();
    let ticks: Vec<u32> = trio
        .host_events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::PeerOrdered {
                message: Msg::Position { tick, .. },
                ..
            } => Some(*tick),
            _ => None,
        })
        .collect();
    assert_eq!(ticks, vec![5, 6]);
    assert!(trio.first.send_ordered(&Msg::Chat("x".to_owned())).is_err());
}

#[test]
fn an_invalid_message_is_not_sent() {
    let mut trio = Trio::connected();
    let too_long = Msg::Chat("x".repeat(MAX_CHAT_BYTES + 1));
    assert!(trio.first.send_relayed(&too_long).is_err());
}

#[test]
fn a_different_protocol_version_is_rejected_on_both_sides() {
    let network = LoopbackNetwork::new();
    let mut host: Session<LoopbackTransport, Current> =
        Session::new(network.endpoint(), config(Role::Host, "Host"));
    let mut guest: Session<LoopbackTransport, Older> =
        Session::new(network.endpoint(), config(Role::Guest, "Guest"));
    let invite = host.host().expect("invite");
    guest.join(&invite).expect("join");
    let mut host_events = Vec::new();
    let mut guest_events = Vec::new();
    let now = Instant::now();
    for _ in 0..6 {
        host.poll(now, &mut host_events);
        guest.poll(now, &mut guest_events);
    }
    assert!(guest_events.contains(&SessionEvent::Rejected {
        reason: RejectReason::VersionMismatch
    }));
    assert!(host_events.iter().any(|event| matches!(
        event,
        SessionEvent::PeerRefused {
            reason: RejectReason::VersionMismatch,
            ..
        }
    )));
    assert!(host.ready_peers().is_empty());
}

#[test]
fn a_third_guest_is_refused_because_the_session_is_full() {
    let mut trio = Trio::connected();
    let mut late: Session<LoopbackTransport, Current> =
        Session::new(trio.network.endpoint(), config(Role::Guest, "Late"));
    let invite = trio.host.host().expect("invite");
    late.join(&invite).expect("join");
    let mut late_events = Vec::new();
    for _ in 0..4 {
        trio.pump();
        late.poll(Instant::now(), &mut late_events);
    }
    assert!(late_events.contains(&SessionEvent::Rejected {
        reason: RejectReason::SessionFull
    }));
}

#[test]
fn a_bye_ends_the_peer_cleanly() {
    let mut trio = Trio::connected();
    trio.second.close();
    trio.pump();
    assert!(trio.host_events.iter().any(|event| matches!(
        event,
        SessionEvent::PeerLeft {
            reason: LeaveReason::Quit,
            ..
        }
    )));
}
