use nc_wire::{GameMessage, LinkPayload};
use replicant_proto::{
    MAX_DISPLAY_NAME_BYTES, MAX_PLAYERS, Member, Message, PROTOCOL_VERSION, Welcome, WireError,
    decode, encode,
};

const FIRST_MEMBER_VARIANT_TAG: u8 = 104;

fn a_member() -> Member {
    Member {
        slot: 1,
        name: "Kaine".to_owned(),
    }
}

#[test]
fn the_member_messages_leave_the_protocol_number_alone() {
    assert_eq!(PROTOCOL_VERSION, 56);
}

#[test]
fn the_member_messages_come_after_every_existing_variant() {
    let joined = encode(&Message::Member(a_member())).expect("encodes");
    let left = encode(&Message::MemberLeft { slot: 1 }).expect("encodes");
    assert_eq!(joined[0], FIRST_MEMBER_VARIANT_TAG);
    assert_eq!(left, vec![FIRST_MEMBER_VARIANT_TAG + 1, 1]);
}

#[test]
fn a_member_joined_and_left_round_trip_and_are_not_relayed() {
    for message in [Message::Member(a_member()), Message::MemberLeft { slot: 0 }] {
        assert!(!message.is_relayed());
        let bytes = encode(&message).expect("encodes");
        assert_eq!(decode(&bytes), Ok(message));
    }
}

#[test]
fn the_member_messages_are_link_payloads() {
    let joined = LinkPayload::Member(a_member());
    let left = LinkPayload::MemberLeft { slot: 1 };
    for payload in [joined, left] {
        let message = Message::from_link(payload.clone());
        assert_eq!(message.into_link(), Ok(payload));
    }
}

#[test]
fn a_member_outside_the_session_or_without_a_good_name_is_refused() {
    let outside = Message::Member(Member {
        slot: MAX_PLAYERS,
        ..a_member()
    });
    assert_eq!(
        encode(&outside),
        Err(WireError::OutOfRange { field: "slot" })
    );
    assert_eq!(
        encode(&Message::MemberLeft { slot: MAX_PLAYERS }),
        Err(WireError::OutOfRange { field: "slot" })
    );
    for name in [String::new(), "x".repeat(MAX_DISPLAY_NAME_BYTES + 1)] {
        let unnamed = Message::Member(Member { name, ..a_member() });
        assert_eq!(
            encode(&unnamed),
            Err(WireError::OutOfRange { field: "name" })
        );
    }
}

#[test]
fn the_member_kinds_have_their_names() {
    assert_eq!(Message::Member(a_member()).kind_name(), "Member");
    assert_eq!(Message::MemberLeft { slot: 1 }.kind_name(), "MemberLeft");
}

fn a_welcome_for(slot: u8) -> Message {
    Message::Welcome(Welcome {
        version: PROTOCOL_VERSION,
        slot,
        host_name: "Host".to_owned(),
    })
}

#[test]
fn a_session_holds_exactly_two_players() {
    assert_eq!(MAX_PLAYERS, 2);
    assert_eq!(MAX_PLAYERS, nc_wire::MAX_PLAYERS_PER_SESSION);
}

#[test]
fn a_welcome_slot_above_the_cap_is_refused() {
    for slot in MAX_PLAYERS..=u8::MAX {
        assert_eq!(
            encode(&a_welcome_for(slot)),
            Err(WireError::OutOfRange { field: "slot" }),
            "slot {slot}"
        );
    }
    for slot in 0..MAX_PLAYERS {
        let bytes = encode(&a_welcome_for(slot)).expect("a slot under the cap encodes");
        assert_eq!(decode(&bytes), Ok(a_welcome_for(slot)));
    }
}
