use rc_game::pickups::{
    ItemCountTable, TurnInSplit, is_shared_item, partner_take_amount, quest_check_count,
    split_turn_in,
};
use rc_proto::{ItemCountEntry, ItemCounts, MAX_ITEM_COUNT_ENTRIES};

const ORDINARY_ITEM: u16 = 111;
const KEY_ITEM: u16 = 0x105;

fn replacing(entries: &[(u16, u8)]) -> ItemCounts {
    ItemCounts {
        replaces_all: true,
        entries: entries
            .iter()
            .map(|(item_id, count)| ItemCountEntry {
                item_id: *item_id,
                count: *count,
            })
            .collect(),
    }
}

#[test]
fn the_partners_count_is_added_for_an_ordinary_item() {
    let mut partner = ItemCountTable::default();
    partner.apply(&replacing(&[(ORDINARY_ITEM, 1)]));
    assert_eq!(
        quest_check_count(2, partner.count(ORDINARY_ITEM)),
        3,
        "host 2 + guest 1 meets a quest needing 3"
    );
}

#[test]
fn a_key_item_never_counts_for_the_partner() {
    let mut partner = ItemCountTable::default();
    partner.apply(&replacing(&[(KEY_ITEM, 1)]));
    assert!(!is_shared_item(KEY_ITEM));
    assert_eq!(partner.count(KEY_ITEM), 0);
}

#[test]
fn a_cleared_table_adds_nothing() {
    let mut partner = ItemCountTable::default();
    partner.apply(&replacing(&[(ORDINARY_ITEM, 5)]));
    partner.clear();
    assert_eq!(quest_check_count(2, partner.count(ORDINARY_ITEM)), 2);
}

#[test]
fn a_replacing_message_drops_what_the_partner_no_longer_holds() {
    let mut partner = ItemCountTable::default();
    partner.apply(&replacing(&[(ORDINARY_ITEM, 5), (80, 2)]));
    partner.apply(&replacing(&[(80, 1)]));
    assert_eq!(partner.count(ORDINARY_ITEM), 0);
    assert_eq!(partner.count(80), 1);
}

#[test]
fn a_single_entry_changes_only_its_item() {
    let mut partner = ItemCountTable::default();
    partner.apply(&replacing(&[(ORDINARY_ITEM, 5), (80, 2)]));
    partner.apply(&ItemCounts {
        replaces_all: false,
        entries: vec![ItemCountEntry {
            item_id: 80,
            count: 0,
        }],
    });
    assert_eq!(partner.count(ORDINARY_ITEM), 5);
    assert_eq!(partner.count(80), 0);
}

#[test]
fn the_sum_stops_at_the_byte_the_game_compares() {
    assert_eq!(quest_check_count(200, 99), u8::MAX);
}

#[test]
fn a_turn_in_the_local_stock_covers_takes_nothing_from_the_partner() {
    assert_eq!(
        split_turn_in(3, 5, 4),
        TurnInSplit {
            local: 3,
            partner: 0
        }
    );
}

#[test]
fn a_turn_in_the_local_stock_cannot_cover_asks_the_partner_for_the_rest() {
    assert_eq!(
        split_turn_in(3, 2, 4),
        TurnInSplit {
            local: 2,
            partner: 1
        }
    );
}

#[test]
fn a_turn_in_is_clamped_when_the_partner_is_short_too() {
    assert_eq!(
        split_turn_in(5, 1, 2),
        TurnInSplit {
            local: 1,
            partner: 2
        }
    );
}

#[test]
fn the_partner_removes_no_more_than_it_holds() {
    assert_eq!(partner_take_amount(4, 1), 1);
    assert_eq!(partner_take_amount(1, 4), 1);
}

#[test]
fn a_full_sync_splits_a_large_table_and_rebuilds_it() {
    let mut mine = ItemCountTable::default();
    let wanted = MAX_ITEM_COUNT_ENTRIES * 2 + 5;
    for item_id in 0..wanted {
        mine.set(item_id as u16, 7);
    }
    let messages = mine.full_sync();
    assert_eq!(messages.len(), 3);
    assert!(messages[0].replaces_all);
    assert!(messages[1..].iter().all(|message| !message.replaces_all));
    assert!(messages.iter().all(|message| message.validate().is_ok()));
    let mut theirs = ItemCountTable::default();
    theirs.set(500, 9);
    for message in &messages {
        theirs.apply(message);
    }
    assert_eq!(theirs.count(500), 0);
    assert_eq!(theirs.count((wanted - 1) as u16), 7);
}

#[test]
fn an_empty_table_still_sends_one_replacing_message() {
    let messages = ItemCountTable::default().full_sync();
    assert_eq!(messages.len(), 1);
    assert!(messages[0].replaces_all && messages[0].entries.is_empty());
}
