use automata_proto::{
    MAX_DISPLAY_NAME_BYTES, MAX_PLAYERS, Member, Message, Welcome, WireError, decode, encode,
};
use nc_wire::{GameMessage, LinkPayload};

const FIRST_MEMBER_VARIANT_TAG: u8 = 22;

const STORY_RULES: nc_wire::SessionRules = nc_wire::SessionRules {
    mode: nc_wire::STORY_MODE,
    max_players: 2,
    map: 0,
};

fn a_member() -> Member {
    Member {
        slot: 1,
        name: "Partner".to_owned(),
    }
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

#[test]
fn a_session_holds_up_to_the_wire_ceiling() {
    assert_eq!(MAX_PLAYERS, 16);
    assert_eq!(MAX_PLAYERS, nc_wire::MAX_SESSION_SLOTS);
}

#[test]
fn a_welcome_slot_at_or_above_the_cap_is_refused() {
    let welcome_for = |slot| {
        Message::Welcome(Welcome {
            version: automata_proto::PROTOCOL_VERSION,
            slot,
            host_name: "Host".to_owned(),
            rules: STORY_RULES,
        })
    };
    for slot in MAX_PLAYERS..=u8::MAX {
        assert_eq!(
            encode(&welcome_for(slot)),
            Err(WireError::OutOfRange { field: "slot" }),
            "slot {slot}"
        );
    }
    for slot in 0..MAX_PLAYERS {
        let bytes = encode(&welcome_for(slot)).expect("a slot under the cap encodes");
        assert_eq!(decode(&bytes), Ok(welcome_for(slot)));
    }
}

#[test]
fn rules_travel_as_a_link_payload_and_are_checked() {
    let rules = Message::Rules(STORY_RULES);
    let bytes = encode(&rules).expect("encodes");
    assert_eq!(decode(&bytes), Ok(rules.clone()));
    assert_eq!(
        rules.clone().into_link(),
        Ok(LinkPayload::Rules(STORY_RULES))
    );
    assert_eq!(Message::from_link(LinkPayload::Rules(STORY_RULES)), rules);
    assert!(!rules.is_relayed());
    assert_eq!(rules.kind_name(), "Rules");
    let free_roam = Message::Rules(nc_wire::SessionRules {
        mode: 1,
        max_players: MAX_PLAYERS,
        map: 0,
    });
    assert!(encode(&free_roam).is_ok());
    for max_players in [0, 1, MAX_PLAYERS + 1] {
        let rejected = Message::Rules(nc_wire::SessionRules {
            mode: 1,
            max_players,
            map: 0,
        });
        assert_eq!(
            encode(&rejected),
            Err(WireError::OutOfRange {
                field: "max_players"
            })
        );
    }
}
