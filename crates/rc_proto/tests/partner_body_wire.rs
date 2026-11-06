use rc_proto::{
    MAX_MESSAGE_BYTES, Message, MotionState, NO_MOTION_NAME_HASH, PartnerBody, decode, encode,
    motion_name_hash,
};

const NIER_IDLE_NAME: &str = "NIER013_idle_lp";
const KNOWN_FNV1_HASH_OF_A: u32 = 0x050C_5D7E;
const UNKNOWN_BODY_BYTE: u8 = 2;

#[test]
fn a_body_choice_survives_a_round_trip_in_one_small_message() {
    for body in [PartnerBody::Nier, PartnerBody::PrologueCharacter] {
        let message = Message::PartnerBody(body);
        let bytes = encode(&message).expect("a body choice encodes");
        assert!(bytes.len() <= MAX_MESSAGE_BYTES);
        assert_eq!(decode(&bytes), Ok(message));
    }
}

#[test]
fn a_body_choice_is_relayed_between_the_games() {
    assert!(Message::PartnerBody(PartnerBody::Nier).is_relayed());
}

#[test]
fn a_body_byte_nobody_defined_is_refused() {
    let mut bytes = encode(&Message::PartnerBody(PartnerBody::Nier)).expect("encodes");
    let last = bytes.len() - 1;
    bytes[last] = UNKNOWN_BODY_BYTE;
    assert!(decode(&bytes).is_err());
}

#[test]
fn the_body_defaults_to_nier() {
    assert_eq!(PartnerBody::default(), PartnerBody::Nier);
}

#[test]
fn the_motion_name_hash_is_fnv1_and_never_the_none_value() {
    assert_eq!(motion_name_hash("a"), KNOWN_FNV1_HASH_OF_A);
    assert_ne!(motion_name_hash(NIER_IDLE_NAME), NO_MOTION_NAME_HASH);
    assert_ne!(motion_name_hash(""), NO_MOTION_NAME_HASH);
}

#[test]
fn a_motion_carries_its_name_hash_over_the_wire() {
    let motion = MotionState {
        index: 3,
        frame: 1.5,
        speed: 1.0,
        looping: false,
        name_hash: motion_name_hash(NIER_IDLE_NAME),
    };
    let bytes = postcard::to_stdvec(&motion).expect("encodes");
    let back: MotionState = postcard::from_bytes(&bytes).expect("decodes");
    assert_eq!(back, motion);
}
