use rc_proto::{
    BodyPackName, MAX_BODY_PACK_NAME_BYTES, Message, MotionState, NO_MOTION_NAME_HASH, PartnerBody,
    decode, encode, motion_name_hash,
};

const NIER_IDLE_NAME: &str = "NIER013_idle_lp";
const KNOWN_FNV1_HASH_OF_A: u32 = 0x050C_5D7E;

#[test]
fn a_body_pack_name_survives_a_round_trip_in_one_small_message() {
    for name in ["NIER013", "NIER014", "a", "SIXTEEN_BYTES_OK"] {
        let message = Message::PartnerBody(BodyPackName::new(name).expect("valid"));
        let bytes = encode(&message).expect("a body pack name encodes");
        assert!(bytes.len() <= MAX_BODY_PACK_NAME_BYTES + 4);
        assert_eq!(decode(&bytes), Ok(message));
    }
}

#[test]
fn a_body_pack_name_is_relayed_between_the_games() {
    let pack = BodyPackName::new("NIER013").expect("valid");
    assert!(Message::PartnerBody(pack).is_relayed());
}

#[test]
fn a_body_pack_name_that_is_empty_long_or_not_plain_ascii_is_refused() {
    for name in [
        "",
        "SEVENTEEN_BYTES_X",
        "NIER 013",
        "NIER\u{e9}",
        "NIER\n13",
        "a/b",
    ] {
        assert!(BodyPackName::new(name).is_err(), "{name:?}");
    }
}

#[test]
fn a_bad_body_pack_name_on_the_wire_is_refused_on_decode() {
    let good = encode(&Message::PartnerBody(
        BodyPackName::new("ABC").expect("valid"),
    ))
    .expect("ok");
    let mut bytes = good.clone();
    let last = bytes.len() - 1;
    bytes[last] = b' ';
    assert!(decode(&bytes).is_err());
    assert!(decode(&good).is_ok());
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
