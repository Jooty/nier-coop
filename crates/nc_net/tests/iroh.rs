use std::time::{Duration, Instant};

use nc_net::{
    Channel, IrohConfig, IrohTransport, LinkProgress, PathRoute, ProgressBoard, Transport,
    TransportEvent,
};

const WAIT_LIMIT: Duration = Duration::from_secs(20);
const POLL_STEP: Duration = Duration::from_millis(20);

fn local_transport() -> IrohTransport {
    IrohTransport::new(IrohConfig {
        use_relays_and_lookup: false,
    })
    .expect("endpoint binds")
}

fn wait_until(
    host: &mut IrohTransport,
    guest: &mut IrohTransport,
    host_events: &mut Vec<TransportEvent>,
    guest_events: &mut Vec<TransportEvent>,
    done: impl Fn(&[TransportEvent], &[TransportEvent]) -> bool,
) {
    let deadline = Instant::now() + WAIT_LIMIT;
    while Instant::now() < deadline {
        host.poll(host_events);
        guest.poll(guest_events);
        if done(host_events, guest_events) {
            return;
        }
        std::thread::sleep(POLL_STEP);
    }
    panic!("timed out; host saw {host_events:?}, guest saw {guest_events:?}");
}

fn received(events: &[TransportEvent], channel: Channel, expected: &[u8]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            TransportEvent::Message { channel: seen, payload, .. }
                if *seen == channel && payload == expected
        )
    })
}

#[test]
fn two_local_endpoints_connect_and_exchange_both_channels() {
    let mut host = local_transport();
    let mut guest = local_transport();
    let invite = host.host().expect("host issues an invite");
    guest.join(&invite).expect("guest dials");
    let (mut host_events, mut guest_events) = (Vec::new(), Vec::new());
    wait_until(
        &mut host,
        &mut guest,
        &mut host_events,
        &mut guest_events,
        |host_seen, guest_seen| {
            let connected = |events: &[TransportEvent]| {
                events
                    .iter()
                    .any(|e| matches!(e, TransportEvent::PeerConnected(_)))
            };
            connected(host_seen) && connected(guest_seen)
        },
    );
    let guest_id = guest.local_peer();
    host.send(guest_id, Channel::ReliableOrdered, b"reliable hello")
        .expect("reliable send");
    for _ in 0..50 {
        host.send(guest_id, Channel::UnreliableLatest, b"datagram hello")
            .expect("datagram send");
    }
    wait_until(
        &mut host,
        &mut guest,
        &mut host_events,
        &mut guest_events,
        |_, guest_seen| {
            received(guest_seen, Channel::ReliableOrdered, b"reliable hello")
                && received(guest_seen, Channel::UnreliableLatest, b"datagram hello")
        },
    );
    let stats = host.link_stats(guest_id).expect("link has stats");
    assert!(stats.direct);
}

#[test]
fn an_invite_that_is_not_ours_is_refused() {
    let mut guest = local_transport();
    let invite = nc_net::Invite::new("rcn1notbase32!".to_owned()).expect("valid text");
    assert_eq!(guest.join(&invite), Err(nc_net::NetError::InvalidInvite));
}

#[test]
fn only_an_invite_this_transport_wrote_reads_without_connecting() {
    let mut host = local_transport();
    let invite = host.host().expect("invite");
    assert!(nc_net::invite_reads(invite.as_str()));
    assert!(!nc_net::invite_reads("rcn1notbase32!"));
    assert!(!nc_net::invite_reads("loopback-1"));
    assert!(!nc_net::invite_reads(""));
}

#[test]
fn a_local_join_reports_the_link_as_it_comes_up() {
    let mut host = local_transport();
    let mut guest = local_transport();
    let host_board = ProgressBoard::default();
    let guest_board = ProgressBoard::default();
    host.report_progress(host_board.fresh_sink());
    guest.report_progress(guest_board.fresh_sink());
    let invite = host.host().expect("host issues an invite");
    guest.join(&invite).expect("guest dials");
    let (mut host_events, mut guest_events) = (Vec::new(), Vec::new());
    wait_until(
        &mut host,
        &mut guest,
        &mut host_events,
        &mut guest_events,
        |_, _| {
            guest_board
                .entries()
                .iter()
                .any(|entry| matches!(entry.progress, LinkProgress::PathWon { .. }))
        },
    );
    let host_told: Vec<LinkProgress> = host_board
        .entries()
        .into_iter()
        .map(|e| e.progress)
        .collect();
    assert!(matches!(
        host_told[0],
        LinkProgress::Bound {
            ipv4_port: Some(_),
            ..
        }
    ));
    assert!(matches!(host_told[1], LinkProgress::OwnId(_)));
    assert!(
        host_told
            .iter()
            .any(|progress| matches!(progress, LinkProgress::DirectAddressFound { count: 1, .. })),
        "the ticket waited for a direct address: {host_told:?}"
    );
    let guest_told: Vec<LinkProgress> = guest_board
        .entries()
        .into_iter()
        .map(|e| e.progress)
        .collect();
    let place = |wanted: fn(&LinkProgress) -> bool| {
        guest_told
            .iter()
            .position(wanted)
            .unwrap_or_else(|| panic!("missing from {guest_told:?}"))
    };
    let read = place(
        |p| matches!(p, LinkProgress::TicketRead { relay: None, direct_addresses, .. } if *direct_addresses > 0),
    );
    let answered = place(|p| matches!(p, LinkProgress::HostAnswered { .. }));
    let won = place(|p| {
        matches!(
            p,
            LinkProgress::PathWon {
                route: PathRoute::Direct(_),
                ..
            }
        )
    });
    assert!(read < answered && answered < won, "{guest_told:?}");
}
