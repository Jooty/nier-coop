use rc_proto::{
    ContainerFlag, ContainerItem, ContainerOpened, MAX_CONTAINER_FLAGS, MAX_CONTAINER_GOLD,
    MAX_CONTAINER_ITEMS, MAX_MESSAGE_BYTES, Message, StoryArray, decode, encode,
};

fn opened() -> ContainerOpened {
    ContainerOpened {
        phase: 12,
        action_id: 7051,
        items: vec![ContainerItem {
            item_id: 21,
            count: 2,
        }],
        gold: 300,
        flags: vec![ContainerFlag {
            array: StoryArray::GameFlags,
            id: 1355,
        }],
    }
}

fn full() -> ContainerOpened {
    ContainerOpened {
        phase: 12,
        action_id: 7051,
        items: vec![
            ContainerItem {
                item_id: 0x2FF,
                count: 99
            };
            MAX_CONTAINER_ITEMS
        ],
        gold: MAX_CONTAINER_GOLD,
        flags: vec![
            ContainerFlag {
                array: StoryArray::SnowFlags,
                id: 0x7FF
            };
            MAX_CONTAINER_FLAGS
        ],
    }
}

#[test]
fn a_container_notice_survives_a_round_trip_and_is_relayed() {
    let message = Message::ContainerOpened(opened());
    let bytes = encode(&message).expect("valid notice encodes");
    assert_eq!(decode(&bytes), Ok(message.clone()));
    assert!(message.is_relayed());
}

#[test]
fn the_largest_container_notice_fits_one_message() {
    let bytes = encode(&Message::ContainerOpened(full())).expect("full notice encodes");
    assert!(bytes.len() <= MAX_MESSAGE_BYTES, "{} bytes", bytes.len());
}

#[test]
fn nonsense_container_notices_are_refused() {
    let mut too_many_items = opened();
    too_many_items.items = full().items;
    too_many_items.items.push(too_many_items.items[0]);
    let mut zero_count = opened();
    zero_count.items[0].count = 0;
    let mut rich = opened();
    rich.gold = MAX_CONTAINER_GOLD + 1;
    let mut counter_flag = opened();
    counter_flag.flags[0].array = StoryArray::GameCounters;
    let mut flag_too_high = opened();
    flag_too_high.flags[0].id = 0x800;
    for bad in [
        too_many_items,
        zero_count,
        rich,
        counter_flag,
        flag_too_high,
    ] {
        assert!(bad.validate().is_err(), "{bad:?}");
        assert!(encode(&Message::ContainerOpened(bad)).is_err());
    }
}
