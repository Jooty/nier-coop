use automata_game::pickups::{
    GivenItem, ItemPickup, MAX_ITEM_ID, find_item_record, heard_pickup, heard_pickup_line,
    is_key_item_kind, is_told_give, local_pickup_line, merge_given_items,
};

const HOST_SLOT: u8 = 0;
const GUEST_SLOT: u8 = 1;

fn ordinary(item_id: u32, count: u16) -> ItemPickup {
    ItemPickup {
        item_id,
        count,
        key_item: false,
    }
}

fn key(item_id: u32) -> ItemPickup {
    ItemPickup {
        item_id,
        count: 1,
        key_item: true,
    }
}

#[test]
fn a_pickup_told_to_us_is_logged_with_the_players_name() {
    let heard = heard_pickup(GUEST_SLOT, Some(HOST_SLOT), "Ana", ordinary(80, 1)).expect("heard");
    assert_eq!(heard.sender_name, "Ana");
    assert_eq!(
        heard_pickup_line(&heard.sender_name, Some("Bolt"), heard.pickup, None),
        "Ana picked up Bolt"
    );
    assert_eq!(
        heard_pickup_line("Ana", Some("Bolt"), ordinary(80, 3), None),
        "Ana picked up Bolt x3"
    );
    assert_eq!(
        heard_pickup_line("Ana", None, ordinary(80, 1), None),
        "Ana picked up item 80"
    );
}

#[test]
fn our_own_pickup_is_not_logged_twice() {
    assert_eq!(
        heard_pickup(GUEST_SLOT, Some(GUEST_SLOT), "Me", ordinary(80, 1)),
        None
    );
    assert_eq!(local_pickup_line(Some("Bolt"), ordinary(80, 1)), None);
}

#[test]
fn a_pickup_does_not_remove_the_item_from_another_players_world() {
    let heard = heard_pickup(HOST_SLOT, Some(GUEST_SLOT), "Host", ordinary(80, 1)).expect("heard");
    assert_eq!(heard.grant_item_id, None);
    assert_eq!(heard.pickup, ordinary(80, 1));
}

#[test]
fn a_key_item_is_given_to_the_hearer_too_and_said_once_by_its_finder() {
    let heard = heard_pickup(HOST_SLOT, Some(GUEST_SLOT), "Host", key(501)).expect("heard");
    assert_eq!(heard.grant_item_id, Some(501));
    assert_eq!(
        heard_pickup_line("Host", Some("Key"), key(501), Some(true)),
        "Host picked up Key (key item), you have it too"
    );
    assert_eq!(
        heard_pickup_line("Host", Some("Key"), key(501), Some(false)),
        "Host picked up Key (key item)"
    );
    assert_eq!(
        local_pickup_line(Some("Key"), key(501)).as_deref(),
        Some("Key is a key item, the others get it too")
    );
}

#[test]
fn a_pickup_out_of_range_is_not_heard() {
    assert_eq!(
        heard_pickup(
            HOST_SLOT,
            Some(GUEST_SLOT),
            "Host",
            ordinary(MAX_ITEM_ID + 1, 1)
        ),
        None
    );
    assert_eq!(
        heard_pickup(HOST_SLOT, Some(GUEST_SLOT), "Host", ordinary(80, 0)),
        None
    );
}

#[test]
fn only_a_give_the_game_added_and_showed_is_told() {
    assert!(is_told_give(1, 1));
    assert!(!is_told_give(0, 1));
    assert!(!is_told_give(1, 0));
}

#[test]
fn items_given_in_one_frame_are_told_once_each_with_a_count() {
    let given = [
        GivenItem {
            item_id: 80,
            key_item: false,
        },
        GivenItem {
            item_id: 501,
            key_item: true,
        },
        GivenItem {
            item_id: 80,
            key_item: false,
        },
    ];
    assert_eq!(merge_given_items(&given), vec![ordinary(80, 2), key(501)]);
}

#[test]
fn the_item_record_is_found_in_the_sorted_table() {
    let ids = [3, 10, 80, 501, 4000];
    let id_at = |index: i32| ids.get(usize::try_from(index).ok()?).copied();
    assert_eq!(find_item_record(5, 501, id_at), Some(3));
    assert_eq!(find_item_record(5, 3, id_at), Some(0));
    assert_eq!(find_item_record(5, 81, id_at), None);
    assert_eq!(find_item_record(0, 80, id_at), None);
    assert!(is_key_item_kind(6));
    assert!(!is_key_item_kind(0));
}
