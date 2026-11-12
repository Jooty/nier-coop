use std::thread;
use std::time::{Duration, Instant};

use nc_hook::net::{
    LinkAccess, LinkPeer, NetGame, NetState, NetThread, RemotePlayer, first_state_tick,
};
use nc_net::{
    Channel, LoopbackNetwork, LoopbackTransport, Role, Session, SessionConfig, SessionEvent,
    Transport, TransportEvent,
};
use nc_wire::{
    BuildGate, GameMessage, Hello, LinkPayload, Member, Ping, Pong, Reject, Welcome, WireError,
    decode, encode,
};
use serde::{Deserialize, Serialize};

const WAIT_LIMIT: Duration = Duration::from_secs(20);
const MADE_UP_PROTOCOL_VERSION: u16 = 7;
const A_SLOT: u8 = 1;
const B_SLOT: u8 = 0;
const A_GUARD_NAME: &str = "Mira";
const B_GUARD_NAME: &str = "Odo";
const A_LANTERN_X: i32 = 5;
const B_LANTERN_X: i32 = 9;
const LANTERN_X_OF_A_COUNT_THAT_STARTS_AT_ONE: i32 = 77;
const PAUSE_BEFORE_THE_WORKER_STARTS_AGAIN: Duration = Duration::from_millis(100);
const SESSION_PUMP_ROUNDS: usize = 6;
const MADE_UP_BUILD: BuildGate = BuildGate {
    time_date_stamp: 7,
    size_of_image: 9,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum LanternMessage {
    Hello(Hello),
    Welcome(Welcome),
    Reject(Reject),
    Ping(Ping),
    Pong(Pong),
    Bye,
    Lantern { tick: u32, x: i32 },
    Member(Member),
    MemberLeft { slot: u8 },
}

impl GameMessage for LanternMessage {
    const PROTOCOL_VERSION: u16 = MADE_UP_PROTOCOL_VERSION;
    const MAX_PLAYERS: u8 = 2;

    fn from_link(payload: LinkPayload) -> Self {
        match payload {
            LinkPayload::Hello(hello) => Self::Hello(hello),
            LinkPayload::Welcome(welcome) => Self::Welcome(welcome),
            LinkPayload::Reject(reject) => Self::Reject(reject),
            LinkPayload::Ping(ping) => Self::Ping(ping),
            LinkPayload::Pong(pong) => Self::Pong(pong),
            LinkPayload::Bye => Self::Bye,
            LinkPayload::Member(member) => Self::Member(member),
            LinkPayload::MemberLeft { slot } => Self::MemberLeft { slot },
        }
    }

    fn into_link(self) -> Result<LinkPayload, Self> {
        match self {
            Self::Hello(hello) => Ok(LinkPayload::Hello(hello)),
            Self::Welcome(welcome) => Ok(LinkPayload::Welcome(welcome)),
            Self::Reject(reject) => Ok(LinkPayload::Reject(reject)),
            Self::Ping(ping) => Ok(LinkPayload::Ping(ping)),
            Self::Pong(pong) => Ok(LinkPayload::Pong(pong)),
            Self::Bye => Ok(LinkPayload::Bye),
            Self::Member(member) => Ok(LinkPayload::Member(member)),
            Self::MemberLeft { slot } => Ok(LinkPayload::MemberLeft { slot }),
            other @ Self::Lantern { .. } => Err(other),
        }
    }

    fn is_relayed(&self) -> bool {
        false
    }

    fn is_latest_wins(&self) -> bool {
        matches!(self, Self::Lantern { .. })
    }

    fn ordered_tick(&self) -> Option<u32> {
        match self {
            Self::Lantern { tick, .. } => Some(*tick),
            _ => None,
        }
    }

    fn validate(&self) -> Result<(), WireError> {
        Ok(())
    }
}

struct LanternGame;

impl NetGame for LanternGame {
    type Message = LanternMessage;
    type Snapshot = i32;
    type PeerState = i32;
    type Extras = ();

    fn kind_name(message: &LanternMessage) -> &'static str {
        match message {
            LanternMessage::Lantern { .. } => "Lantern",
            _ => "Link",
        }
    }

    fn peer_in_play(_extras: &()) -> bool {
        true
    }

    fn snapshot_message(&mut self, snapshot: &i32, tick: u32) -> LanternMessage {
        LanternMessage::Lantern { tick, x: *snapshot }
    }

    fn on_message(&mut self, link: &LinkAccess<'_, Self>, message: LanternMessage) {
        let LanternMessage::Lantern { x, .. } = message else {
            return;
        };
        let mut shared = link.shared();
        let name = shared.peer_name.clone().unwrap_or_default();
        let slot = u8::from(shared.local_slot == Some(0));
        shared.remote = Some(RemotePlayer {
            name,
            slot,
            state: x,
            received_at: Instant::now(),
        });
    }

    fn on_connected(&mut self, _extras: &mut (), _peer_name: &str) {}

    fn on_peer_cleared(&mut self, _extras: &mut ()) {}

    fn on_launcher_linked(&mut self) {}

    fn on_idle(
        &mut self,
        _link: &LinkAccess<'_, Self>,
        _send: &mut dyn FnMut(&LanternMessage, Channel) -> bool,
    ) {
    }

    fn announce(&self, _text: &str) {}
}

struct LauncherPair {
    host: Session<LoopbackTransport, LanternMessage>,
    guest: Session<LoopbackTransport, LanternMessage>,
    host_events: Vec<SessionEvent<LanternMessage>>,
}

impl LauncherPair {
    fn connected() -> Self {
        let network = LoopbackNetwork::new();
        let mut host = Session::new(
            network.endpoint(),
            SessionConfig {
                role: Role::Host,
                name: B_GUARD_NAME.to_owned(),
                build: MADE_UP_BUILD,
            },
        );
        let mut guest = Session::new(
            network.endpoint(),
            SessionConfig {
                role: Role::Guest,
                name: A_GUARD_NAME.to_owned(),
                build: MADE_UP_BUILD,
            },
        );
        let invite = host.host().expect("invite");
        guest.join(&invite).expect("join");
        let mut pair = Self {
            host,
            guest,
            host_events: Vec::new(),
        };
        pair.pump();
        pair
    }

    fn pump(&mut self) {
        let now = Instant::now();
        let mut guest_events = Vec::new();
        for _ in 0..SESSION_PUMP_ROUNDS {
            self.host.poll(now, &mut self.host_events);
            self.guest.poll(now, &mut guest_events);
        }
    }

    fn forward_from_the_guest(&mut self, message: &LanternMessage) {
        self.guest.send_ordered(message).expect("an ordered kind");
        self.pump();
    }

    fn lanterns_the_host_took(&self) -> Vec<i32> {
        self.host_events
            .iter()
            .filter_map(|event| match event {
                SessionEvent::PeerOrdered {
                    message: LanternMessage::Lantern { x, .. },
                    ..
                } => Some(*x),
                _ => None,
            })
            .collect()
    }
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

fn welcome(slot: u8, host_name: &str) -> LanternMessage {
    LanternMessage::Welcome(Welcome {
        version: MADE_UP_PROTOCOL_VERSION,
        slot,
        host_name: host_name.to_owned(),
    })
}

fn start_pair() -> (NetThread<LanternGame>, NetThread<LanternGame>) {
    let network = LoopbackNetwork::new();
    let a_transport = network.endpoint();
    let mut b_transport = network.endpoint();
    let a_peer = a_transport.local_peer();
    let b_peer = b_transport.local_peer();
    let b_invite = b_transport.host().expect("b hosts");
    let a = NetThread::spawn(
        LanternGame,
        a_transport,
        LinkPeer {
            invite: Some(b_invite),
            peer: b_peer,
            port: 1,
        },
    )
    .expect("a starts");
    let b = NetThread::spawn(
        LanternGame,
        b_transport,
        LinkPeer {
            invite: None,
            peer: a_peer,
            port: 2,
        },
    )
    .expect("b starts");
    (a, b)
}

fn start_connected_worker_with_its_launcher_end() -> (NetThread<LanternGame>, LoopbackTransport) {
    let network = LoopbackNetwork::new();
    let worker_transport = network.endpoint();
    let mut launcher_end = network.endpoint();
    let worker_peer = worker_transport.local_peer();
    let invite = launcher_end.host().expect("the launcher end hosts");
    let worker = NetThread::spawn(
        LanternGame,
        worker_transport,
        LinkPeer {
            invite: Some(invite),
            peer: launcher_end.local_peer(),
            port: 1,
        },
    )
    .expect("the worker starts");
    assert!(wait_until(
        || worker.view().state == NetState::LauncherLinked
    ));
    let welcome = encode(&welcome(A_SLOT, B_GUARD_NAME)).expect("a welcome encodes");
    launcher_end
        .send(worker_peer, Channel::ReliableOrdered, &welcome)
        .expect("the welcome is sent");
    assert!(wait_until(|| worker.view().state == NetState::Connected));
    (worker, launcher_end)
}

fn first_lantern_sent_by(
    worker: &NetThread<LanternGame>,
    launcher_end: &mut LoopbackTransport,
    x: i32,
) -> LanternMessage {
    let mut sent = None;
    assert!(wait_until(|| {
        worker.publish(x);
        let mut events = Vec::new();
        launcher_end.poll(&mut events);
        sent = events.into_iter().find_map(|event| match event {
            TransportEvent::Message { payload, .. } => decode::<LanternMessage>(&payload)
                .ok()
                .filter(|message| matches!(message, LanternMessage::Lantern { .. })),
            _ => None,
        });
        sent.is_some()
    }));
    sent.expect("the wait saw a lantern")
}

#[test]
fn the_states_of_a_worker_started_again_are_not_dropped_as_old() {
    let mut launchers = LauncherPair::connected();
    let (first_worker, mut first_launcher_end) = start_connected_worker_with_its_launcher_end();
    let from_the_first_run =
        first_lantern_sent_by(&first_worker, &mut first_launcher_end, A_LANTERN_X);
    drop(first_worker);
    thread::sleep(PAUSE_BEFORE_THE_WORKER_STARTS_AGAIN);
    let (second_worker, mut second_launcher_end) = start_connected_worker_with_its_launcher_end();
    let from_the_second_run =
        first_lantern_sent_by(&second_worker, &mut second_launcher_end, B_LANTERN_X);
    let from_a_count_that_starts_at_one = LanternMessage::Lantern {
        tick: 1,
        x: LANTERN_X_OF_A_COUNT_THAT_STARTS_AT_ONE,
    };

    launchers.forward_from_the_guest(&from_the_first_run);
    launchers.forward_from_the_guest(&from_a_count_that_starts_at_one);
    launchers.forward_from_the_guest(&from_the_second_run);

    assert_eq!(
        launchers.lanterns_the_host_took(),
        vec![A_LANTERN_X, B_LANTERN_X]
    );
}

#[test]
fn the_state_count_starts_later_for_every_later_start() {
    let first_start = Duration::from_secs(1_700_000_000);
    let started_again = first_start + PAUSE_BEFORE_THE_WORKER_STARTS_AGAIN;
    let newer_by = first_state_tick(started_again).wrapping_sub(first_state_tick(first_start));
    assert!(newer_by as i32 > 0);
}

#[test]
fn the_state_count_wraps_instead_of_stopping_at_the_largest_number() {
    let step = Duration::from_millis(16);
    let at_the_largest = step * u32::MAX;
    assert_eq!(first_state_tick(at_the_largest), u32::MAX);
    assert_eq!(first_state_tick(at_the_largest + step), 0);
}

#[test]
fn two_workers_link_and_each_receives_the_others_message() {
    let (a, b) = start_pair();
    assert!(wait_until(|| {
        a.view().state == NetState::LauncherLinked && b.view().state == NetState::LauncherLinked
    }));

    a.send_to_launcher(welcome(A_SLOT, A_GUARD_NAME));
    b.send_to_launcher(welcome(B_SLOT, B_GUARD_NAME));
    assert!(wait_until(|| {
        a.view().state == NetState::Connected && b.view().state == NetState::Connected
    }));
    assert_eq!(a.view().state.label(), "connected");
    assert_eq!(b.view().state.label(), "connected");
    assert_eq!(a.view().peer_name.as_deref(), Some(B_GUARD_NAME));
    assert_eq!(b.view().peer_name.as_deref(), Some(A_GUARD_NAME));

    a.publish(A_LANTERN_X);
    b.publish(B_LANTERN_X);
    assert!(wait_until(|| {
        a.view()
            .remote
            .is_some_and(|remote| remote.state == B_LANTERN_X)
            && b.view()
                .remote
                .is_some_and(|remote| remote.state == A_LANTERN_X)
    }));
}

#[test]
fn a_worker_waits_for_the_link_before_it_is_connected() {
    let (a, b) = start_pair();
    assert!(wait_until(|| b.view().state == NetState::LauncherLinked));
    assert_eq!(b.view().state.label(), "launcher linked, no other player");
    assert!(!a.wants_snapshots());
    assert!(!b.wants_snapshots());
}
