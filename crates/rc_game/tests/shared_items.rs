use rc_game::pickups::{
    INJECTED_END_TAKE_BLOCK_MS, InjectedEndWindow, ItemCountTable, PARTNER_END_WAIT_CAP_MS,
    PartnerTakeQueue, PollHold, QueuedTakeFlushReason, TurnInSplit, is_menu_quest_page_caller,
    is_shared_item, item_poll_runs_here, partner_take_amount, quest_check_count, split_turn_in,
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
fn a_queued_take_is_taken_inside_the_partner_end() {
    let mut queue = PartnerTakeQueue::new();
    queue.queue(ORDINARY_ITEM, 2, PARTNER_END, 1_000);
    assert!(queue.is_waiting());
    assert_eq!(queue.take_in_end(ORDINARY_ITEM, "SEID_other_End01"), None);
    assert_eq!(queue.take_in_end(ORDINARY_ITEM + 1, PARTNER_END), None);
    assert_eq!(queue.take_in_end(ORDINARY_ITEM, PARTNER_END), Some(2));
    assert!(!queue.is_waiting());
    assert_eq!(queue.take_in_end(ORDINARY_ITEM, PARTNER_END), None);
}

#[test]
fn a_queued_take_applies_at_the_cap_when_the_end_never_arrives() {
    let mut queue = PartnerTakeQueue::new();
    queue.queue(ORDINARY_ITEM, 2, PARTNER_END, 1_000);
    assert!(
        queue
            .flush_due(1_000 + PARTNER_END_WAIT_CAP_MS - 1, |_| false)
            .is_empty()
    );
    let flushed = queue.flush_due(1_000 + PARTNER_END_WAIT_CAP_MS, |_| false);
    assert_eq!(flushed.len(), 1);
    assert_eq!(flushed[0].reason, QueuedTakeFlushReason::Cap);
    assert_eq!(flushed[0].take.amount, 2);
    assert!(!queue.is_waiting());
}

#[test]
fn a_queued_take_applies_when_the_end_finished_without_a_take() {
    let mut queue = PartnerTakeQueue::new();
    queue.queue(ORDINARY_ITEM, 2, PARTNER_END, 1_000);
    assert!(queue.flush_due(5_000, |_| false).is_empty());
    let flushed = queue.flush_due(25_000, |name| name == PARTNER_END);
    assert_eq!(flushed.len(), 1);
    assert_eq!(
        flushed[0].reason,
        QueuedTakeFlushReason::EndFinishedWithoutTake
    );
    assert_eq!(flushed[0].waited_ms, 24_000);
}

#[test]
fn a_queued_take_is_dropped_when_the_partner_leaves() {
    let mut queue = PartnerTakeQueue::new();
    queue.queue(ORDINARY_ITEM, 2, PARTNER_END, 1_000);
    queue.clear();
    assert!(!queue.is_waiting());
    assert!(queue.flush_due(99_000, |_| true).is_empty());
}

#[test]
fn queued_takes_of_one_item_for_one_end_add_up() {
    let mut queue = PartnerTakeQueue::new();
    queue.queue(ORDINARY_ITEM, 2, PARTNER_END, 1_000);
    queue.queue(ORDINARY_ITEM, 1, PARTNER_END, 2_000);
    assert_eq!(queue.amount_waiting_for(ORDINARY_ITEM, PARTNER_END), 3);
    assert_eq!(queue.take_in_end(ORDINARY_ITEM, PARTNER_END), Some(3));
}

#[test]
fn only_the_pause_menu_quest_page_return_address_counts_as_a_quest_check() {
    assert!(is_menu_quest_page_caller(0x4D_7770));
    assert!(!is_menu_quest_page_caller(0x41_30FD));
    assert!(!is_menu_quest_page_caller(0));
}
