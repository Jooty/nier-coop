use nt_net::{
    Channel, Invite, LoopbackNetwork, MAX_PAYLOAD_BYTES, NetError, PeerId, Transport,
    TransportEvent,
};

fn connected_pair() -> (nt_net::LoopbackTransport, nt_net::LoopbackTransport) {
    let network = LoopbackNetwork::new();
    let mut host = network.endpoint();
    let mut guest = network.endpoint();
    let invite = host.host().expect("host issues an invite");
    guest.join(&invite).expect("guest joins");
    (host, guest)
}

#[test]
fn joining_connects_both_sides() {
    let (mut host, mut guest) = connected_pair();
    let (mut host_events, mut guest_events) = (Vec::new(), Vec::new());
    host.poll(&mut host_events);
    guest.poll(&mut guest_events);
    assert_eq!(
        host_events,
        vec![TransportEvent::PeerConnected(guest.local_peer())]
    );
    assert_eq!(
        guest_events,
        vec![TransportEvent::PeerConnected(host.local_peer())]
    );
}

#[test]
fn messages_arrive_in_send_order_on_the_channel_they_were_sent() {
    let (mut host, mut guest) = connected_pair();
    host.send(guest.local_peer(), Channel::ReliableOrdered, b"one")
        .expect("sends");
    host.send(guest.local_peer(), Channel::UnreliableLatest, b"two")
        .expect("sends");
    let mut events = Vec::new();
    guest.poll(&mut events);
    let messages: Vec<_> = events
        .into_iter()
        .filter_map(|event| match event {
            TransportEvent::Message {
                channel, payload, ..
            } => Some((channel, payload)),
            _ => None,
        })
        .collect();
    assert_eq!(
        messages,
        vec![
            (Channel::ReliableOrdered, b"one".to_vec()),
            (Channel::UnreliableLatest, b"two".to_vec()),
        ]
    );
}

#[test]
fn sending_to_a_stranger_or_an_oversized_payload_is_refused() {
    let (mut host, guest) = connected_pair();
    assert_eq!(
        host.send(PeerId([9; 32]), Channel::ReliableOrdered, b"x"),
        Err(NetError::NotConnected)
    );
    let too_big = vec![0u8; MAX_PAYLOAD_BYTES + 1];
    assert!(matches!(
        host.send(guest.local_peer(), Channel::ReliableOrdered, &too_big),
        Err(NetError::PayloadTooLarge { .. })
    ));
}

#[test]
fn an_invite_nobody_issued_is_refused() {
    let network = LoopbackNetwork::new();
    let mut guest = network.endpoint();
    let invite = Invite::new("loopback-99".to_owned()).expect("valid text");
    assert_eq!(guest.join(&invite), Err(NetError::InvalidInvite));
}

#[test]
fn an_invite_is_bounded() {
    assert_eq!(Invite::new(String::new()), Err(NetError::InvalidInvite));
    assert_eq!(
        Invite::new("x".repeat(nt_net::MAX_INVITE_BYTES + 1)),
        Err(NetError::InvalidInvite)
    );
}

#[test]
fn disconnecting_notifies_both_sides_and_stops_delivery() {
    let (mut host, mut guest) = connected_pair();
    let guest_id = guest.local_peer();
    host.disconnect(guest_id);
    let mut events = Vec::new();
    guest.poll(&mut events);
    assert!(events.iter().any(|event| matches!(
        event,
        TransportEvent::PeerDisconnected(peer, _) if *peer == host.local_peer()
    )));
    assert_eq!(
        host.send(guest_id, Channel::ReliableOrdered, b"late"),
        Err(NetError::NotConnected)
    );
}
