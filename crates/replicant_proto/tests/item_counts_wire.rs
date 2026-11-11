use replicant_proto::{
    ItemCountEntry, ItemCounts, ItemTake, MAX_ITEM_COUNT_ENTRIES, MAX_ITEM_ID, MAX_MESSAGE_BYTES,
    Message, decode, encode,
};

#[test]
fn item_counts_round_trip_and_are_relayed() {
    let message = Message::ItemCounts(ItemCounts {
        replaces_all: true,
        entries: vec![
            ItemCountEntry {
                item_id: 79,
                count: 3,
            },
            ItemCountEntry {
                item_id: MAX_ITEM_ID,
                count: 0,
            },
        ],
    });
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn a_full_chunk_of_item_counts_fits_in_one_message() {
    let entries = (0..MAX_ITEM_COUNT_ENTRIES)
        .map(|index| ItemCountEntry {
            item_id: MAX_ITEM_ID - index as u16,
            count: 99,
        })
        .collect();
    let message = Message::ItemCounts(ItemCounts {
        replaces_all: false,
        entries,
    });
    let bytes = encode(&message).unwrap();
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes), Ok(message));
}

#[test]
fn item_counts_with_too_many_entries_or_a_bad_id_are_refused() {
    let too_many = Message::ItemCounts(ItemCounts {
        replaces_all: false,
        entries: vec![
            ItemCountEntry {
                item_id: 1,
                count: 1
            };
            MAX_ITEM_COUNT_ENTRIES + 1
        ],
    });
    assert!(too_many.validate().is_err());
    let bad_id = Message::ItemCounts(ItemCounts {
        replaces_all: false,
        entries: vec![ItemCountEntry {
            item_id: MAX_ITEM_ID + 1,
            count: 1,
        }],
    });
    assert!(bad_id.validate().is_err());
}

#[test]
fn an_item_take_round_trips_and_is_relayed() {
    let message = Message::ItemTake(ItemTake {
        item_id: 111,
        count: 4,
    });
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn an_item_take_of_nothing_is_refused() {
    let message = Message::ItemTake(ItemTake {
        item_id: 111,
        count: 0,
    });
    assert!(message.validate().is_err());
}
