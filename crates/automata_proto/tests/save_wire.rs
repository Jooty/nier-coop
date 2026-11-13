use automata_proto::{Message, decode, encode};
use nc_wire::GameMessage;

const HOST_SAVED_VARIANT_TAG: u8 = 33;

fn a_host_save() -> Message {
    Message::HostSaved { number: 7 }
}

#[test]
fn the_host_saved_message_comes_after_every_existing_variant() {
    let bytes = encode(&a_host_save()).expect("encodes");
    assert_eq!(bytes[0], HOST_SAVED_VARIANT_TAG);
    assert_eq!(a_host_save().kind_name(), "HostSaved");
}

#[test]
fn the_host_saved_message_round_trips() {
    for number in [0, 1, u32::MAX] {
        let message = Message::HostSaved { number };
        let bytes = encode(&message).expect("encodes");
        assert_eq!(decode(&bytes).expect("decodes"), message);
    }
}

#[test]
fn the_host_saved_message_is_relayed_reliable_and_not_latest_wins() {
    let message = a_host_save();
    assert!(message.is_relayed());
    assert!(!message.is_latest_wins());
    assert!(!GameMessage::forwarded_to_other_guests(&message));
    assert_eq!(message.validate(), Ok(()));
}
