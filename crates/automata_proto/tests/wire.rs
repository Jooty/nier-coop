use automata_game::character::Character;
use automata_game::partner::BodySnapshot;
use automata_proto::{
    BuildGate, CharacterHello, Hello, MAX_MESSAGE_BYTES, MAX_PLAYERS, Message, PROTOCOL_VERSION,
    PathInfo, Ping, PlayerState, Pong, Reject, RejectReason, Welcome, WireError, decode, encode,
};
use nc_wire::{
    GameMessage, GameReadiness, LinkPayload, LobbyCharacter, LobbyMessage, LobbyPayload,
    LobbyReady, LobbyStart,
};

const BUILD: BuildGate = BuildGate {
    time_date_stamp: 0x6AC8_9519,
    size_of_image: 0x0100_0000,
};

fn state() -> PlayerState {
    PlayerState {
        sequence: 7,
        held: 0x0000_0101,
        pressed: 0x0000_0100,
        button_mask: 0b0000_0101,
        speed: 3.5,
        world_facing: 1.25,
        position: [1.5, -2.0, 300.25],
        body_facing: -0.5,
        state: 12,
        variant: 2,
        b: 0xFFFF_FFFF,
        c: 3,
        health: 4200,
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
    round_trip(Message::PathInfo(PathInfo {
        rtt_micros: 42_000,
        direct: true,
    }));
    for payload in lobby_payloads() {
        round_trip(Message::from_lobby(payload));
    }
}

fn lobby_payloads() -> [LobbyPayload; 4] {
    [
        LobbyPayload::Ready(LobbyReady {
            ready: true,
            mode: 1,
            slot: Some(2),
        }),
        LobbyPayload::Start(LobbyStart { mode: 0 }),
        LobbyPayload::Character(LobbyCharacter { index: 2 }),
        LobbyPayload::GameReadiness(GameReadiness::InPlay),
    ]
}

#[test]
fn lobby_payloads_map_both_ways_and_are_relayed() {
    for payload in lobby_payloads() {
        let message = Message::from_lobby(payload);
        assert!(message.is_relayed());
        assert!(!message.is_latest_wins());
        assert_eq!(message.into_lobby(), Ok(payload));
    }
    assert!(Message::State(state()).into_lobby().is_err());
}

#[test]
fn a_lobby_message_out_of_range_is_refused() {
    let start = Message::LobbyStart(LobbyStart { mode: 9 });
    assert_eq!(encode(&start), Err(WireError::OutOfRange { field: "mode" }));
}

#[test]
fn the_messages_the_hooks_already_send_keep_their_wire_numbers() {
    let hello = Message::Character(CharacterHello {
        character: Character::TwoB,
    });
    assert_eq!(encode(&hello).expect("encodes")[0], 6);
    assert_eq!(encode(&Message::State(state())).expect("encodes")[0], 7);
    let report = Message::PathInfo(PathInfo {
        rtt_micros: 1,
        direct: true,
    });
    assert_eq!(encode(&report).expect("encodes")[0], 8);
}

#[test]
fn a_state_carries_every_field_of_the_snapshot_both_ways() {
    let snapshot = BodySnapshot {
        held: 3,
        pressed: 1,
        button_mask: 0x81,
        speed: 6.0,
        world_facing: 2.0,
        position: [10.0, 20.0, 30.0],
        body_facing: 1.0,
        state: 0x44,
        variant: 1,
        b: 2,
        c: 3,
        health: 900,
    };
    let state = PlayerState::from_snapshot(&snapshot, 65_537);
    assert_eq!(state.sequence, 65_537);
    assert_eq!(state.to_snapshot(), snapshot);
    let Ok(Message::State(decoded)) = decode(&encode(&Message::State(state)).expect("encodes"))
    else {
        panic!("expected a state message");
    };
    assert_eq!(decoded.to_snapshot(), snapshot);
}

#[test]
fn the_path_report_is_not_relayed_to_the_other_player() {
    let report = Message::PathInfo(PathInfo {
        rtt_micros: 1,
        direct: false,
    });
    assert!(!report.is_relayed());
    assert!(!report.is_latest_wins());
}

#[test]
fn the_protocol_version_is_in_this_games_range() {
    assert_eq!(PROTOCOL_VERSION, 1002);
    assert_eq!(<Message as GameMessage>::PROTOCOL_VERSION, 1002);
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
    let mut world_facing = state();
    world_facing.world_facing = 99.0;
    assert_eq!(
        encode(&Message::State(world_facing)),
        Err(WireError::OutOfRange {
            field: "world_facing"
        })
    );
    let mut body_facing = state();
    body_facing.body_facing = f32::NAN;
    assert_eq!(
        encode(&Message::State(body_facing)),
        Err(WireError::OutOfRange {
            field: "body_facing"
        })
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
