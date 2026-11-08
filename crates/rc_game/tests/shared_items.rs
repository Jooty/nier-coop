use rc_game::pickups::{
    INJECTED_END_TAKE_BLOCK_MS, InjectedEndWindow, ItemCountTable, PARTNER_END_WAIT_CAP_MS,
    PollHold, TurnInSplit, is_shared_item, item_poll_runs_here, partner_take_amount,
    quest_check_count, split_turn_in,
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

#[test]
fn the_partners_count_is_lowered_in_the_table_before_the_take() {
    let mut partner = ItemCountTable::default();
    partner.apply(&replacing(&[(ORDINARY_ITEM, 2)]));
    let split = split_turn_in(3, 1, partner.count(ORDINARY_ITEM));
    partner.lower(ORDINARY_ITEM, split.partner);
    assert_eq!(partner.count(ORDINARY_ITEM), 0);
    partner.lower(ORDINARY_ITEM, 5);
    assert_eq!(partner.count(ORDINARY_ITEM), 0, "never below zero");
}

#[test]
fn the_item_poll_runs_on_the_host_or_when_the_partner_is_away() {
    assert!(item_poll_runs_here(true, true));
    assert!(!item_poll_runs_here(false, true));
    assert!(item_poll_runs_here(false, false));
    assert!(item_poll_runs_here(true, false));
}

#[test]
fn the_poll_sees_the_frozen_count_for_two_seconds_after_a_take() {
    let mut hold = PollHold::new();
    assert_eq!(hold.frozen_count(ORDINARY_ITEM, 0), None);
    hold.note_take(ORDINARY_ITEM, 3, 10_000);
    assert_eq!(hold.frozen_count(ORDINARY_ITEM, 11_999), Some(3));
    assert_eq!(hold.frozen_count(ORDINARY_ITEM, 12_000), None);
    assert_eq!(hold.frozen_count(ORDINARY_ITEM + 1, 10_500), None);
}

#[test]
fn an_injected_end_blocks_takes_only_inside_its_window() {
    let mut window = InjectedEndWindow::new();
    assert!(!window.is_open(0));
    window.open(5_000);
    assert!(window.is_open(5_000 + INJECTED_END_TAKE_BLOCK_MS - 1));
    assert!(!window.is_open(5_000 + INJECTED_END_TAKE_BLOCK_MS));
}

const PARTNER_END: &str = "SEID_0100_S0010_a0050_End01";

#[test]
fn a_partner_take_holds_the_poll_until_the_partner_end_ran_here() {
    let mut hold = PollHold::new();
    let waits = hold.note_partner_take(ORDINARY_ITEM, 2, 3, 1_000, Some((PARTNER_END, false)));
    assert_eq!(waits.as_deref(), Some(PARTNER_END));
    assert_eq!(hold.frozen_by_partner_wait(ORDINARY_ITEM), Some(3));
    assert_eq!(hold.frozen_by_partner_wait(ORDINARY_ITEM + 1), None);
    assert_eq!(hold.partner_wait_end(), Some(PARTNER_END));
    assert_eq!(hold.release_partner_wait(20_000, false), None);
    assert_eq!(hold.frozen_by_partner_wait(ORDINARY_ITEM), Some(3));
    let release = hold.release_partner_wait(24_000, true).expect("released");
    assert_eq!((release.waited_ms, release.by_cap), (23_000, false));
    assert_eq!(hold.frozen_by_partner_wait(ORDINARY_ITEM), None);
}

#[test]
fn no_hold_when_the_partner_end_ran_here_before_the_take() {
    let mut hold = PollHold::new();
    let waits = hold.note_partner_take(ORDINARY_ITEM, 2, 3, 1_000, Some((PARTNER_END, true)));
    assert_eq!(waits, None);
    assert_eq!(hold.frozen_by_partner_wait(ORDINARY_ITEM), None);
    assert_eq!(hold.frozen_count(ORDINARY_ITEM, 1_500), None);
}

#[test]
fn the_partner_end_wait_gives_up_at_the_cap() {
    let mut hold = PollHold::new();
    hold.note_partner_take(ORDINARY_ITEM, 2, 3, 1_000, Some((PARTNER_END, false)));
    assert_eq!(
        hold.release_partner_wait(1_000 + PARTNER_END_WAIT_CAP_MS - 1, false),
        None
    );
    let release = hold
        .release_partner_wait(1_000 + PARTNER_END_WAIT_CAP_MS, false)
        .expect("released");
    assert!(release.by_cap);
    assert_eq!(hold.frozen_by_partner_wait(ORDINARY_ITEM), None);
}

#[test]
fn the_partner_end_wait_ends_when_the_partner_leaves() {
    let mut hold = PollHold::new();
    hold.note_partner_take(ORDINARY_ITEM, 2, 3, 1_000, Some((PARTNER_END, false)));
    hold.clear_partner_wait();
    assert_eq!(hold.frozen_by_partner_wait(ORDINARY_ITEM), None);
    assert_eq!(hold.release_partner_wait(2_000, true), None);
}

#[test]
fn a_partner_take_without_a_known_end_keeps_the_two_second_hold() {
    let mut hold = PollHold::new();
    assert_eq!(
        hold.note_partner_take(ORDINARY_ITEM, 2, 3, 1_000, None),
        None
    );
    assert_eq!(hold.frozen_count(ORDINARY_ITEM, 1_500), Some(3));
}
