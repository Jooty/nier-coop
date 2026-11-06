use rc_proto::{
    BuildGate, Hello, MAX_DISPLAY_NAME_BYTES, MAX_MESSAGE_BYTES, Message, PROTOCOL_VERSION, Ping,
    PlayerState, Reject, RejectReason, Welcome, WireError, decode, encode,
};

const BUILD: BuildGate = BuildGate {
    time_date_stamp: 0x60B7_3C00,
    size_of_image: 0x0533_9000,
};

fn state() -> PlayerState {
    PlayerState {
        tick: 7,
        phase: 3,
        position: [1.5, -2.0, 300.25],
        yaw_radians: 1.25,
        animation_id: 12,
        animation_seconds: 0.5,
        hp: 80,
        max_hp: 100,
    }
}

fn hello() -> Hello {
    Hello {
        version: PROTOCOL_VERSION,
        build: BUILD,
        name: "Nier".to_owned(),
    }
}

fn round_trip(message: Message) {
    let bytes = encode(&message).expect("valid message encodes");
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes), Ok(message));
}

#[test]
fn every_message_kind_survives_a_round_trip() {
    round_trip(Message::Hello(hello()));
    round_trip(Message::Welcome(Welcome {
        version: PROTOCOL_VERSION,
        slot: 1,
        host_name: "Host".to_owned(),
    }));
    round_trip(Message::Reject(Reject {
        reason: RejectReason::BuildMismatch,
    }));
    round_trip(Message::State(state()));
    round_trip(Message::Ping(Ping { nonce: 9 }));
    round_trip(Message::Bye);
    round_trip(Message::PathInfo(rc_proto::PathInfo {
        rtt_micros: 1500,
        direct: true,
    }));
    round_trip(Message::Challenge(rc_proto::Challenge { nonce: [7; 16] }));
    round_trip(Message::Proof(rc_proto::Proof { mac: [9; 32] }));
    round_trip(Message::Reject(Reject {
        reason: RejectReason::WrongPassword,
    }));
}

#[test]
fn a_state_message_fits_far_below_the_size_limit() {
    let bytes = encode(&Message::State(state())).expect("encodes");
    assert!(bytes.len() < MAX_MESSAGE_BYTES / 4);
}

#[test]
fn an_empty_message_is_refused() {
    assert_eq!(decode(&[]), Err(WireError::Empty));
}

#[test]
fn an_oversized_message_is_refused_before_parsing() {
    let bytes = vec![0u8; MAX_MESSAGE_BYTES + 1];
    assert!(matches!(decode(&bytes), Err(WireError::TooLarge { .. })));
}

#[test]
fn every_truncation_of_a_valid_message_is_refused() {
    let bytes = encode(&Message::State(state())).expect("encodes");
    for len in 1..bytes.len() {
        assert!(
            decode(&bytes[..len]).is_err(),
            "prefix of {len} bytes decoded"
        );
    }
}

#[test]
fn bytes_after_the_message_are_refused() {
    let mut bytes = encode(&Message::Bye).expect("encodes");
    bytes.push(0);
    assert_eq!(decode(&bytes), Err(WireError::TrailingBytes { extra: 1 }));
}

#[test]
fn an_unknown_message_tag_is_refused() {
    assert_eq!(decode(&[0xFF, 0x01]), Err(WireError::Malformed));
}

#[test]
fn garbage_never_panics_and_never_decodes_to_an_invalid_message() {
    let mut seed: u32 = 0x1234_5678;
    for _ in 0..2000 {
        let len = {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 24) as usize % 64 + 1
        };
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (seed >> 24) as u8
            })
            .collect();
        if let Ok(message) = decode(&bytes) {
            assert_eq!(message.validate(), Ok(()));
        }
    }
}

#[test]
fn out_of_range_player_state_is_refused_on_encode_and_decode() {
    let bad_states = [
        PlayerState { hp: 101, ..state() },
        PlayerState {
            position: [f32::NAN, 0.0, 0.0],
            ..state()
        },
        PlayerState {
            position: [0.0, f32::INFINITY, 0.0],
            ..state()
        },
        PlayerState {
            position: [2_000_000.0, 0.0, 0.0],
            ..state()
        },
        PlayerState {
            yaw_radians: 100.0,
            ..state()
        },
        PlayerState {
            animation_seconds: -1.0,
            ..state()
        },
        PlayerState {
            max_hp: u32::MAX,
            hp: 0,
            ..state()
        },
    ];
    for bad in bad_states {
        assert!(matches!(
            encode(&Message::State(bad)),
            Err(WireError::OutOfRange { .. })
        ));
        let raw = postcard::to_stdvec(&Message::State(bad)).expect("serialises");
        assert!(matches!(decode(&raw), Err(WireError::OutOfRange { .. })));
    }
}

#[test]
fn display_names_are_bounded_and_printable() {
    let too_long = "x".repeat(MAX_DISPLAY_NAME_BYTES + 1);
    for name in [too_long.as_str(), "", "tab\there"] {
        let message = Message::Hello(Hello {
            name: name.to_owned(),
            ..hello()
        });
        assert!(matches!(
            encode(&message),
            Err(WireError::OutOfRange { field: "name" })
        ));
    }
    let longest = "x".repeat(MAX_DISPLAY_NAME_BYTES);
    round_trip(Message::Hello(Hello {
        name: longest,
        ..hello()
    }));
}

#[test]
fn a_hello_from_another_protocol_version_still_decodes_so_the_session_can_reject_it() {
    let message = Message::Hello(Hello {
        version: PROTOCOL_VERSION + 1,
        ..hello()
    });
    round_trip(message);
}

#[test]
fn a_welcome_for_a_slot_beyond_the_player_limit_is_refused() {
    let message = Message::Welcome(Welcome {
        version: PROTOCOL_VERSION,
        slot: 2,
        host_name: "Host".to_owned(),
    });
    assert!(matches!(
        encode(&message),
        Err(WireError::OutOfRange { field: "slot" })
    ));
}
