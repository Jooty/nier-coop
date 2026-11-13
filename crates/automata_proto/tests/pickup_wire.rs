use automata_proto::{ItemPickup, Message, WireError, decode, encode};
use nc_wire::GameMessage;

const PICKUP_VARIANT_TAG: u8 = 26;

fn a_pickup() -> Message {
    Message::Pickup(ItemPickup {
        item_id: 80,
        count: 2,
        key_item: false,
    })
}

#[test]
fn a_pickup_is_told_to_every_other_player() {
    let pickup = a_pickup();
    assert!(pickup.is_relayed());
    assert!(GameMessage::forwarded_to_other_guests(&pickup));
    assert!(!pickup.is_latest_wins());
}

#[test]
fn the_pickup_message_comes_after_every_existing_variant() {
    let bytes = encode(&a_pickup()).expect("encodes");
    assert_eq!(bytes[0], PICKUP_VARIANT_TAG);
    assert_eq!(decode(&bytes).expect("decodes"), a_pickup());
    assert_eq!(a_pickup().kind_name(), "Pickup");
}

#[test]
fn a_pickup_without_a_count_or_out_of_range_is_refused() {
    for pickup in [
        ItemPickup {
            item_id: 80,
            count: 0,
            key_item: false,
        },
        ItemPickup {
            item_id: 2_000_000,
            count: 1,
            key_item: true,
        },
    ] {
        assert_eq!(
            Message::Pickup(pickup).validate(),
            Err(WireError::OutOfRange { field: "pickup" })
        );
    }
}
