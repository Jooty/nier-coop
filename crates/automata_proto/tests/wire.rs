use automata_game::character::Character;
use automata_proto::{
    BuildGate, CharacterHello, Hello, MAX_MESSAGE_BYTES, MAX_PLAYERS, Message, PROTOCOL_VERSION,
    Ping, PlayerState, Pong, Reject, RejectReason, Welcome, WireError, decode, encode,
};
use nc_wire::{GameMessage, LinkPayload};

const BUILD: BuildGate = BuildGate {
    time_date_stamp: 0x6AC8_9519,
    size_of_image: 0x0100_0000,
};

fn state() -> PlayerState {
    PlayerState {
        tick: 7,
        position: [1.5, -2.0, 300.25],
        facing: 1.25,
        speed: 3.5,
        state_id: 12,
    }
}

fn round_trip(message: Message) {
    let bytes = encode(&message).expect("valid message encodes");
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes), Ok(message));
}

#[test]
fn every_message_kind_survives_a_round_trip() {
    round_trip(Message::Hello(Hello {
        version: PROTOCOL_VERSION,
        build: BUILD,
        name: "Nier".to_owned(),
    }));
    round_trip(Message::Welcome(Welcome {
        version: PROTOCOL_VERSION,
        slot: 1,
        host_name: "Host".to_owned(),
    }));
    round_trip(Message::Reject(Reject {
        reason: RejectReason::VersionMismatch,
    }));
    round_trip(Message::Ping(Ping { nonce: 9 }));
    round_trip(Message::Pong(Pong { nonce: 9 }));
    round_trip(Message::Bye);
    for character in Character::ALL {
        round_trip(Message::Character(CharacterHello { character }));
    }
    round_trip(Message::State(state()));
}

#[test]
fn the_protocol_version_starts_this_games_range() {
    assert_eq!(PROTOCOL_VERSION, 1001);
    assert_eq!(<Message as GameMessage>::PROTOCOL_VERSION, 1001);
    assert_eq!(<Message as GameMessage>::MAX_PLAYERS, MAX_PLAYERS);
    assert_eq!(MAX_PLAYERS, 2);
}

#[test]
fn only_the_state_message_is_latest_wins_and_ordered() {
    let message = Message::State(state());
    assert!(message.is_latest_wins());
    assert_eq!(GameMessage::ordered_tick(&message), Some(7));
    let hello = Message::Character(CharacterHello {
        character: Character::A2,
    });
    assert!(!hello.is_latest_wins());
    assert_eq!(GameMessage::ordered_tick(&hello), None);
}

#[test]
fn link_payloads_map_both_ways() {
    for payload in [
        LinkPayload::Ping(Ping { nonce: 3 }),
        LinkPayload::Pong(Pong { nonce: 3 }),
        LinkPayload::Bye,
    ] {
        let message = Message::from_link(payload.clone());
        assert_eq!(message.into_link(), Ok(payload));
    }
    let game = Message::State(state());
    assert_eq!(game.clone().into_link(), Err(game));
}

#[test]
fn game_messages_are_relayed_and_link_messages_are_not() {
    assert!(Message::State(state()).is_relayed());
    assert!(
        Message::Character(CharacterHello {
            character: Character::NineS
        })
        .is_relayed()
    );
    assert!(!Message::Bye.is_relayed());
    assert!(!Message::Ping(Ping { nonce: 1 }).is_relayed());
}

#[test]
fn a_state_message_fits_far_below_the_size_limit() {
    let bytes = encode(&Message::State(state())).expect("encodes");
    assert!(bytes.len() < MAX_MESSAGE_BYTES / 4);
}

#[test]
fn a_non_finite_or_huge_position_is_refused() {
    for bad in [f32::NAN, f32::INFINITY, 2_000_000.0] {
        let mut broken = state();
        broken.position[1] = bad;
        assert_eq!(
            encode(&Message::State(broken)),
            Err(WireError::OutOfRange { field: "position" })
        );
    }
}

#[test]
fn a_bad_facing_or_speed_is_refused() {
    let mut facing = state();
    facing.facing = 99.0;
    assert_eq!(
        encode(&Message::State(facing)),
        Err(WireError::OutOfRange { field: "facing" })
    );
    let mut speed = state();
    speed.speed = f32::NAN;
    assert_eq!(
        encode(&Message::State(speed)),
        Err(WireError::OutOfRange { field: "speed" })
    );
}

#[test]
fn a_welcome_with_a_slot_beyond_the_player_count_is_refused() {
    let message = Message::Welcome(Welcome {
        version: PROTOCOL_VERSION,
        slot: MAX_PLAYERS,
        host_name: "Host".to_owned(),
    });
    assert_eq!(
        encode(&message),
        Err(WireError::OutOfRange { field: "slot" })
    );
}

#[test]
fn an_empty_or_cut_message_is_refused() {
    assert_eq!(decode(&[]), Err(WireError::Empty));
    let bytes = encode(&Message::State(state())).expect("encodes");
    assert_eq!(decode(&bytes[..bytes.len() - 1]), Err(WireError::Truncated));
}

#[test]
fn bytes_after_a_message_are_refused() {
    let mut bytes = encode(&Message::Bye).expect("encodes");
    bytes.push(0);
    assert_eq!(decode(&bytes), Err(WireError::TrailingBytes { extra: 1 }));
}
