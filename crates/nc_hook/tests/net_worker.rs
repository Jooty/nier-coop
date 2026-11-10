use std::thread;
use std::time::{Duration, Instant};

use nc_hook::net::{LinkAccess, LinkPeer, NetGame, NetState, NetThread, RemotePlayer};
use nc_net::{Channel, LoopbackNetwork, Transport};
use nc_wire::{GameMessage, Hello, LinkPayload, Ping, Pong, Reject, Welcome, WireError};
use serde::{Deserialize, Serialize};

const WAIT_LIMIT: Duration = Duration::from_secs(20);
const MADE_UP_PROTOCOL_VERSION: u16 = 7;
const A_SLOT: u8 = 1;
const B_SLOT: u8 = 0;
const A_GUARD_NAME: &str = "Mira";
const B_GUARD_NAME: &str = "Odo";
const A_LANTERN_X: i32 = 5;
const B_LANTERN_X: i32 = 9;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum LanternMessage {
    Hello(Hello),
    Welcome(Welcome),
    Reject(Reject),
    Ping(Ping),
    Pong(Pong),
    Bye,
    Lantern { tick: u32, x: i32 },
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
