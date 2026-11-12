use std::time::Instant;

use nc_net::{
    Channel, LeaveReason, LoopbackNetwork, LoopbackTransport, PeerId, Role, Session, SessionConfig,
    SessionEvent, Transport, TransportEvent,
};
use nc_wire::{
    BuildGate, Envelope, GameMessage, Hello, LinkPayload, MAX_PLAYERS_PER_SESSION, Member, Ping,
    Pong, Reject, RejectReason, Welcome, WireError, decode, decode_enveloped, encode,
    encode_enveloped,
};
use serde::{Deserialize, Serialize};

const BUILD: BuildGate = BuildGate {
    time_date_stamp: 7,
    size_of_image: 9,
};
const THREE_PLAYERS: u8 = 3;
const MAX_CHAT_BYTES: usize = 16;
const GUEST_ENVELOPE: Envelope = Envelope { from: 1, to: None };

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Msg<const VERSION: u16, const PLAYERS: u8> {
    Hello(Hello),
    Welcome(Welcome),
    Reject(Reject),
    Ping(Ping),
    Pong(Pong),
    Bye,
    Position { tick: u32, x: i32 },
    Chat(String),
    LocalOnly(u8),
    Member(Member),
    MemberLeft { slot: u8 },
}

impl<const VERSION: u16, const PLAYERS: u8> GameMessage for Msg<VERSION, PLAYERS> {
    const PROTOCOL_VERSION: u16 = VERSION;
    const MAX_PLAYERS: u8 = PLAYERS;

    fn from_link(payload: LinkPayload) -> Self {
        match payload {
            LinkPayload::Hello(hello) => Msg::Hello(hello),
            LinkPayload::Welcome(welcome) => Msg::Welcome(welcome),
            LinkPayload::Reject(reject) => Msg::Reject(reject),
            LinkPayload::Ping(ping) => Msg::Ping(ping),
            LinkPayload::Pong(pong) => Msg::Pong(pong),
            LinkPayload::Bye => Msg::Bye,
            LinkPayload::Member(member) => Msg::Member(member),
            LinkPayload::MemberLeft { slot } => Msg::MemberLeft { slot },
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
            Msg::Member(member) => Ok(LinkPayload::Member(member)),
            Msg::MemberLeft { slot } => Ok(LinkPayload::MemberLeft { slot }),
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

type Current = Msg<4, THREE_PLAYERS>;
type Older = Msg<3, THREE_PLAYERS>;
type Pair = Msg<4, MAX_PLAYERS_PER_SESSION>;

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

fn relayed_chats<const VERSION: u16, const PLAYERS: u8>(
    events: &[SessionEvent<Msg<VERSION, PLAYERS>>],
) -> Vec<(u8, String)> {
    events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::Message {
                from,
                message: Msg::Chat(text),
            } => Some((*from, text.clone())),
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
            SessionEvent::PlayerJoined { slot, .. } => Some(*slot),
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
    assert!(
        !trio
            .host_events
            .iter()
            .any(|event| matches!(event, SessionEvent::Message { .. }))
    );
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
    let local_only = encode_enveloped(&GUEST_ENVELOPE, &Current::LocalOnly(3)).expect("encodes");
    for frame in [encode(&hello).expect("encodes"), local_only] {
        raw.send(host_id, Channel::ReliableOrdered, &frame)
            .expect("sends");
        host.poll(now, &mut events);
    }
    assert!(events.iter().any(|event| matches!(
        event,
        SessionEvent::PlayerLeft {
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
            SessionEvent::Message {
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
        SessionEvent::PlayerLeft {
            reason: LeaveReason::Quit,
            ..
        }
    )));
}

fn pair_session(
    network: &LoopbackNetwork,
    role: Role,
    name: &str,
) -> Session<LoopbackTransport, Pair> {
    Session::new(network.endpoint(), config(role, name))
}

fn raw_guest_says_hello(
    network: &LoopbackNetwork,
    host: &mut Session<LoopbackTransport, Pair>,
    version: u16,
    host_events: &mut Vec<SessionEvent<Pair>>,
) -> (LoopbackTransport, PeerId) {
    let mut raw = network.endpoint();
    let invite = host.host().expect("invite");
    raw.join(&invite).expect("join");
    let hello = Pair::Hello(Hello {
        version,
        build: BUILD,
        name: "Raw".to_owned(),
    });
    let host_id = host.transport().local_peer();
    raw.send(
        host_id,
        Channel::ReliableOrdered,
        &encode(&hello).expect("encodes"),
    )
    .expect("sends");
    host.poll(Instant::now(), host_events);
    (raw, host_id)
}

fn frames_received(raw: &mut LoopbackTransport) -> Vec<Vec<u8>> {
    let mut events = Vec::new();
    raw.poll(&mut events);
    events
        .into_iter()
        .filter_map(|event| match event {
            TransportEvent::Message { payload, .. } => Some(payload),
            _ => None,
        })
        .collect()
}

struct Duo {
    host: Session<LoopbackTransport, Pair>,
    guest: Session<LoopbackTransport, Pair>,
    host_events: Vec<SessionEvent<Pair>>,
    guest_events: Vec<SessionEvent<Pair>>,
}

impl Duo {
    fn connected(network: &LoopbackNetwork) -> Self {
        let mut host = pair_session(network, Role::Host, "Host");
        let mut guest = pair_session(network, Role::Guest, "Guest");
        let invite = host.host().expect("invite");
        guest.join(&invite).expect("join");
        let mut duo = Self {
            host,
            guest,
            host_events: Vec::new(),
            guest_events: Vec::new(),
        };
        duo.pump();
        duo
    }

    fn pump(&mut self) {
        pump(&mut [
            (&mut self.host, &mut self.host_events),
            (&mut self.guest, &mut self.guest_events),
        ]);
    }
}

#[test]
fn a_session_refuses_a_joiner_beyond_the_cap() {
    let network = LoopbackNetwork::new();
    let mut duo = Duo::connected(&network);
    let mut late = pair_session(&network, Role::Guest, "Late");
    let invite = duo.host.host().expect("invite");
    late.join(&invite).expect("join");
    let mut late_events = Vec::new();
    for _ in 0..4 {
        duo.pump();
        late.poll(Instant::now(), &mut late_events);
    }
    assert!(late_events.contains(&SessionEvent::Rejected {
        reason: RejectReason::SessionFull
    }));
    assert!(duo.host_events.iter().any(|event| matches!(
        event,
        SessionEvent::PeerRefused {
            reason: RejectReason::SessionFull,
            ..
        }
    )));
    assert_eq!(duo.host.ready_peers().len(), 1);
    assert_eq!(duo.guest.local_slot(), Some(1));
    assert_eq!(late.local_slot(), None);
}

#[test]
fn two_players_see_each_other_join_by_slot_and_name() {
    let network = LoopbackNetwork::new();
    let duo = Duo::connected(&network);
    assert!(duo.host_events.contains(&SessionEvent::PlayerJoined {
        slot: 1,
        name: "Guest".to_owned(),
        peer: duo.guest.transport().local_peer(),
    }));
    assert!(duo.guest_events.contains(&SessionEvent::PlayerJoined {
        slot: 0,
        name: "Host".to_owned(),
        peer: duo.host.transport().local_peer(),
    }));
    assert_eq!(duo.host.local_slot(), Some(0));
    assert_eq!(duo.guest.local_slot(), Some(1));
}

#[test]
fn a_guest_that_leaves_is_reported_by_its_slot() {
    let network = LoopbackNetwork::new();
    let mut duo = Duo::connected(&network);
    duo.guest.close();
    duo.pump();
    assert!(duo.host_events.iter().any(|event| matches!(
        event,
        SessionEvent::PlayerLeft {
            slot: 1,
            reason: LeaveReason::Quit,
            ..
        }
    )));
}

#[test]
fn a_host_that_leaves_is_reported_as_slot_zero() {
    let network = LoopbackNetwork::new();
    let mut duo = Duo::connected(&network);
    duo.host.close();
    duo.pump();
    assert!(duo.guest_events.iter().any(|event| matches!(
        event,
        SessionEvent::PlayerLeft {
            slot: 0,
            reason: LeaveReason::Quit,
            ..
        }
    )));
}

#[test]
fn messages_carry_the_sender_slot_both_ways() {
    let network = LoopbackNetwork::new();
    let mut duo = Duo::connected(&network);
    duo.guest
        .send_relayed(&Msg::Chat("up".to_owned()))
        .expect("relayed kind");
    duo.host
        .send_relayed(&Msg::Chat("down".to_owned()))
        .expect("relayed kind");
    duo.pump();
    assert_eq!(relayed_chats(&duo.host_events), vec![(1, "up".to_owned())]);
    assert_eq!(
        relayed_chats(&duo.guest_events),
        vec![(0, "down".to_owned())]
    );
}

#[test]
fn a_guest_cannot_send_as_another_slot() {
    let network = LoopbackNetwork::new();
    let mut host = pair_session(&network, Role::Host, "Host");
    let mut events = Vec::new();
    let (mut raw, host_id) = raw_guest_says_hello(&network, &mut host, 4, &mut events);
    let posing_as_the_host = Envelope { from: 0, to: None };
    let frame =
        encode_enveloped(&posing_as_the_host, &Pair::Chat("spoof".to_owned())).expect("encodes");
    raw.send(host_id, Channel::ReliableOrdered, &frame)
        .expect("sends");
    host.poll(Instant::now(), &mut events);
    assert_eq!(relayed_chats(&events), vec![(1, "spoof".to_owned())]);
}

#[test]
fn the_handshake_is_bare_and_later_frames_carry_the_envelope() {
    let network = LoopbackNetwork::new();
    let mut host = pair_session(&network, Role::Host, "Host");
    let mut events = Vec::new();
    let (mut raw, _) = raw_guest_says_hello(&network, &mut host, 4, &mut events);
    let handshake = frames_received(&mut raw);
    assert!(matches!(
        decode::<Pair>(&handshake[0]),
        Ok(Msg::Welcome(Welcome { slot: 1, .. }))
    ));
    host.send_relayed(&Msg::Chat("hi".to_owned()))
        .expect("relayed kind");
    let ready: Vec<_> = frames_received(&mut raw)
        .iter()
        .filter_map(|frame| decode_enveloped::<Pair>(frame).ok())
        .collect();
    assert!(ready.contains(&(Envelope { from: 0, to: None }, Msg::Chat("hi".to_owned()))));
}

#[test]
fn a_frame_for_another_slot_is_not_taken() {
    let network = LoopbackNetwork::new();
    let mut host = pair_session(&network, Role::Host, "Host");
    let mut events = Vec::new();
    let (mut raw, host_id) = raw_guest_says_hello(&network, &mut host, 4, &mut events);
    let for_the_guest_itself = Envelope {
        from: 1,
        to: Some(1),
    };
    let frame =
        encode_enveloped(&for_the_guest_itself, &Pair::Chat("x".to_owned())).expect("encodes");
    raw.send(host_id, Channel::ReliableOrdered, &frame)
        .expect("sends");
    host.poll(Instant::now(), &mut events);
    assert!(relayed_chats(&events).is_empty());
    assert_eq!(host.ready_peers().len(), 1);
}

#[test]
fn an_old_version_is_refused_cleanly() {
    let network = LoopbackNetwork::new();
    let mut host = pair_session(&network, Role::Host, "Host");
    let mut events = Vec::new();
    let (mut raw, _) = raw_guest_says_hello(&network, &mut host, 3, &mut events);
    let mut raw_events = Vec::new();
    raw.poll(&mut raw_events);
    assert!(raw_events.iter().any(|event| matches!(
        event,
        TransportEvent::Message { payload, .. }
            if decode::<Older>(payload)
                == Ok(Msg::Reject(Reject { reason: RejectReason::VersionMismatch }))
    )));
    assert!(
        !raw_events
            .iter()
            .any(|event| matches!(event, TransportEvent::PeerDisconnected(..)))
    );
    assert_eq!(host.refused_peer_version(), Some(3));
    assert!(events.iter().any(|event| matches!(
        event,
        SessionEvent::PeerRefused {
            reason: RejectReason::VersionMismatch,
            ..
        }
    )));
}
