use std::time::{Duration, Instant};

use rc_net::{
    ATTEMPT_WINDOW, Channel, GLOBAL_MAX_FAILURES, LoopbackNetwork, LoopbackTransport, Role,
    Session, SessionConfig, SessionEvent, Transport, TransportEvent, password_proof,
};
use rc_proto::{BuildGate, Hello, Message, PROTOCOL_VERSION, Proof, RejectReason, decode, encode};

const BUILD: BuildGate = BuildGate {
    time_date_stamp: 0x60B7_3C00,
    size_of_image: 0x0533_9000,
};
const PASSWORD: &str = "open sesame";

fn config(role: Role, name: &str) -> SessionConfig {
    SessionConfig {
        role,
        name: name.to_owned(),
        build: BUILD,
    }
}

struct Rig {
    network: LoopbackNetwork,
    host: Session<LoopbackTransport>,
    events: Vec<SessionEvent>,
    now: Instant,
}

impl Rig {
    fn new(password: Option<&str>) -> Self {
        let network = LoopbackNetwork::new();
        let mut host = Session::new(network.endpoint(), config(Role::Host, "Host"));
        host.set_password(password.map(str::to_owned));
        Self {
            network,
            host,
            events: Vec::new(),
            now: Instant::now(),
        }
    }

    fn guest(&mut self, password: Option<&str>) -> (Session<LoopbackTransport>, Vec<SessionEvent>) {
        let invite = self.host.host().expect("invite");
        let mut guest = Session::new(self.network.endpoint(), config(Role::Guest, "Guest"));
        guest.set_password(password.map(str::to_owned));
        guest.join(&invite).expect("join");
        let mut guest_events = Vec::new();
        self.pump(&mut guest, &mut guest_events);
        (guest, guest_events)
    }

    fn pump(
        &mut self,
        guest: &mut Session<LoopbackTransport>,
        guest_events: &mut Vec<SessionEvent>,
    ) {
        for _ in 0..6 {
            self.host.poll(self.now, &mut self.events);
            guest.poll(self.now, guest_events);
        }
    }

    fn raw_guest(&mut self) -> (LoopbackTransport, rc_net::PeerId) {
        let invite = self.host.host().expect("invite");
        let mut raw = self.network.endpoint();
        raw.join(&invite).expect("join");
        let host_peer = self.host.transport().local_peer();
        (raw, host_peer)
    }

    fn challenge_for(
        &mut self,
        raw: &mut LoopbackTransport,
        host_peer: rc_net::PeerId,
    ) -> [u8; 16] {
        let hello = Message::Hello(Hello {
            version: PROTOCOL_VERSION,
            build: BUILD,
            name: "Raw".to_owned(),
        });
        raw.send(
            host_peer,
            Channel::ReliableOrdered,
            &encode(&hello).expect("encodes"),
        )
        .expect("sends");
        for _ in 0..4 {
            self.host.poll(self.now, &mut self.events);
        }
        let mut incoming = Vec::new();
        raw.poll(&mut incoming);
        incoming
            .iter()
            .find_map(|event| match event {
                TransportEvent::Message { payload, .. } => match decode(payload) {
                    Ok(Message::Challenge(challenge)) => Some(challenge.nonce),
                    _ => None,
                },
                _ => None,
            })
            .expect("host sends a challenge")
    }

    fn answer(
        &mut self,
        raw: &mut LoopbackTransport,
        host_peer: rc_net::PeerId,
        mac: [u8; 32],
    ) -> Option<RejectReason> {
        let proof = Message::Proof(Proof { mac });
        raw.send(
            host_peer,
            Channel::ReliableOrdered,
            &encode(&proof).expect("encodes"),
        )
        .expect("sends");
        for _ in 0..4 {
            self.host.poll(self.now, &mut self.events);
        }
        let mut incoming = Vec::new();
        raw.poll(&mut incoming);
        incoming.iter().find_map(|event| match event {
            TransportEvent::Message { payload, .. } => match decode(payload) {
                Ok(Message::Reject(reject)) => Some(reject.reason),
                _ => None,
            },
            _ => None,
        })
    }
}

fn joined(events: &[SessionEvent]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, SessionEvent::PeerJoined { .. }))
}

#[test]
fn the_right_password_joins() {
    let mut rig = Rig::new(Some(PASSWORD));
    let (guest, guest_events) = rig.guest(Some(PASSWORD));
    assert!(joined(&guest_events));
    assert!(joined(&rig.events));
    assert_eq!(guest.local_slot(), Some(1));
}

#[test]
fn a_wrong_password_is_rejected_with_its_reason() {
    let mut rig = Rig::new(Some(PASSWORD));
    let (_guest, guest_events) = rig.guest(Some("nope"));
    assert!(guest_events.contains(&SessionEvent::Rejected {
        reason: RejectReason::WrongPassword
    }));
    assert!(!joined(&rig.events));
}

#[test]
fn a_guest_without_a_password_is_told_one_is_needed() {
    let mut rig = Rig::new(Some(PASSWORD));
    let (_guest, guest_events) = rig.guest(None);
    assert!(guest_events.contains(&SessionEvent::Rejected {
        reason: RejectReason::PasswordRequired
    }));
}

#[test]
fn a_host_without_a_password_ignores_the_guest_password() {
    let mut rig = Rig::new(None);
    let (_guest, guest_events) = rig.guest(Some("anything"));
    assert!(joined(&guest_events));
}

#[test]
fn a_proof_captured_from_one_attempt_fails_on_the_next() {
    let mut rig = Rig::new(Some(PASSWORD));
    let (mut first, host_peer) = rig.raw_guest();
    let first_nonce = rig.challenge_for(&mut first, host_peer);
    let captured = password_proof(PASSWORD, &first_nonce, first.local_peer(), host_peer);
    let (mut second, second_host) = rig.raw_guest();
    let second_nonce = rig.challenge_for(&mut second, second_host);
    assert_ne!(first_nonce, second_nonce);
    assert_eq!(
        rig.answer(&mut second, second_host, captured),
        Some(RejectReason::WrongPassword)
    );
}

#[test]
fn a_proof_is_bound_to_the_guest_that_computed_it() {
    let mut rig = Rig::new(Some(PASSWORD));
    let (mut raw, host_peer) = rig.raw_guest();
    let nonce = rig.challenge_for(&mut raw, host_peer);
    let someone_else = rc_net::PeerId([9; 32]);
    let mac = password_proof(PASSWORD, &nonce, someone_else, host_peer);
    assert_eq!(
        rig.answer(&mut raw, host_peer, mac),
        Some(RejectReason::WrongPassword)
    );
}

#[test]
fn the_password_itself_never_appears_in_the_proof_exchange() {
    let nonce = [3u8; 16];
    let guest = rc_net::PeerId([1; 32]);
    let host = rc_net::PeerId([2; 32]);
    let mac = password_proof(PASSWORD, &nonce, guest, host);
    let wire = encode(&Message::Proof(Proof { mac })).expect("encodes");
    assert!(
        !wire
            .windows(PASSWORD.len())
            .any(|chunk| chunk == PASSWORD.as_bytes())
    );
}

#[test]
fn repeated_wrong_proofs_lock_out_new_guests_until_the_window_passes() {
    let mut rig = Rig::new(Some(PASSWORD));
    for _ in 0..GLOBAL_MAX_FAILURES {
        let (mut raw, host_peer) = rig.raw_guest();
        rig.challenge_for(&mut raw, host_peer);
        rig.answer(&mut raw, host_peer, [0; 32]);
        rig.now += Duration::from_secs(1);
    }
    let (_guest, guest_events) = rig.guest(Some(PASSWORD));
    assert!(guest_events.contains(&SessionEvent::Rejected {
        reason: RejectReason::TooManyAttempts
    }));
}

#[test]
fn the_lockout_lifts_after_the_window() {
    let mut rig = Rig::new(Some(PASSWORD));
    for _ in 0..GLOBAL_MAX_FAILURES {
        let (mut raw, host_peer) = rig.raw_guest();
        rig.challenge_for(&mut raw, host_peer);
        rig.answer(&mut raw, host_peer, [0; 32]);
    }
    rig.now += ATTEMPT_WINDOW + Duration::from_secs(1);
    let (_guest, guest_events) = rig.guest(Some(PASSWORD));
    assert!(joined(&guest_events));
}
