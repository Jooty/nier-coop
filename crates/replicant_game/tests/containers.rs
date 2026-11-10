use replicant_game::containers::{
    ContainerRegistry, FLAG_HOLD_MS, FlagHold, OPEN_WINDOW_MS, OpenWindow, OpenedSet, ReceivePlan,
    SkipReason, lua_argument_offset, lua_integer_argument, notice_text, plan_receive,
    remove_only_message,
};
use replicant_game::stay_together::FollowRole;
use replicant_proto::{ContainerFlag, ContainerItem, ContainerOpened, StoryArray, StoryChange};

fn opened(action_id: u16) -> ContainerOpened {
    ContainerOpened {
        phase: 7,
        action_id,
        items: vec![ContainerItem {
            item_id: 21,
            count: 2,
        }],
        gold: 150,
        flags: vec![ContainerFlag {
            array: StoryArray::GameFlags,
            id: 1355,
        }],
    }
}

#[test]
fn lua_arguments_read_integers_and_numbers() {
    assert_eq!(
        lua_integer_argument(4, [0x8B, 0x1B, 0, 0, 0, 0, 0, 0]),
        7051
    );
    assert_eq!(lua_integer_argument(1, 151.0_f64.to_le_bytes()), 151);
    assert_eq!(lua_integer_argument(2, [9; 8]), 0);
    assert_eq!(lua_argument_offset(0), 0x18);
    assert_eq!(lua_argument_offset(2), 0x18 + 2 * 0x90);
}

#[test]
fn the_glint_is_found_through_the_point_id() {
    let mut registry = ContainerRegistry::default();
    registry.note_registration(7051, 8051, 15);
    registry.note_effect(151, 35, 8051);
    registry.note_effect(999, 12, 8051);
    assert_eq!(registry.glint_handle_of(7051), Some(151));
    assert_eq!(registry.glint_handle_of(7052), None);
}

#[test]
fn gathering_points_are_not_registered() {
    let mut registry = ContainerRegistry::default();
    registry.note_registration(7001, 8001, 16);
    assert!(!registry.is_registered(7001));
    registry.note_registration(7002, 8002, 15);
    assert!(registry.is_registered(7002));
}

#[test]
fn the_window_collects_items_gold_and_flags_once() {
    let mut window = OpenWindow::default();
    assert_eq!(window.begin(7051, 7, 1000), None);
    window.note_item(21, 1, 1100);
    window.note_item(21, 1, 1200);
    window.note_item(0x100, 1, 1300);
    window.note_gold(100, 1400);
    window.note_gold(-50, 1450);
    window.note_flag(StoryArray::GameFlags, 1355, 1500);
    window.note_flag(StoryArray::GameFlags, 1355, 1600);
    assert_eq!(window.take_finished(1000 + OPEN_WINDOW_MS - 1), None);
    let done = window.take_finished(1000 + OPEN_WINDOW_MS).expect("closed");
    assert_eq!(
        done.items,
        vec![ContainerItem {
            item_id: 21,
            count: 2
        }]
    );
    assert_eq!(done.gold, 100);
    assert_eq!(done.flags.len(), 1);
    assert!(done.into_message().is_some());
}

#[test]
fn nothing_is_collected_outside_the_window() {
    let mut window = OpenWindow::default();
    window.note_item(21, 1, 0);
    window.begin(7051, 7, 0);
    window.note_item(21, 1, OPEN_WINDOW_MS + 1);
    let done = window.take_finished(OPEN_WINDOW_MS + 1).expect("closed");
    assert!(done.items.is_empty());
}

#[test]
fn a_use_with_only_flags_sends_nothing() {
    let mut window = OpenWindow::default();
    window.begin(7051, 7, 0);
    window.note_flag(StoryArray::SnowFlags, 1081, 10);
    let done = window.take_finished(OPEN_WINDOW_MS).expect("closed");
    assert_eq!(done.into_message(), None);
}

#[test]
fn a_second_container_closes_the_first() {
    let mut window = OpenWindow::default();
    window.begin(7051, 7, 0);
    window.note_gold(10, 5);
    let first = window.begin(7052, 7, 500).expect("first is handed back");
    assert_eq!(first.action_id, 7051);
    assert_eq!(first.gold, 10);
    assert_eq!(window.begin(7052, 7, 600), None);
}

#[test]
fn the_host_sets_the_guests_flags_and_the_guest_does_not_set_the_hosts() {
    let mut set = OpenedSet::default();
    let ReceivePlan::Apply {
        host_sets, notice, ..
    } = plan_receive(FollowRole::Host, &opened(7051), Some(7), &mut set, "Pat")
    else {
        panic!("host applies");
    };
    assert_eq!(host_sets.len(), 1);
    assert!(notice.is_some());
    let mut guest_set = OpenedSet::default();
    let ReceivePlan::Apply { host_sets, .. } = plan_receive(
        FollowRole::Guest,
        &opened(7051),
        Some(7),
        &mut guest_set,
        "Pat",
    ) else {
        panic!("guest applies");
    };
    assert!(host_sets.is_empty());
}

#[test]
fn another_zone_and_a_repeat_are_skipped() {
    let mut set = OpenedSet::default();
    assert_eq!(
        plan_receive(FollowRole::Host, &opened(7051), Some(8), &mut set, "Pat"),
        ReceivePlan::Skip(SkipReason::OtherZone)
    );
    assert!(matches!(
        plan_receive(FollowRole::Host, &opened(7051), Some(7), &mut set, "Pat"),
        ReceivePlan::Apply { .. }
    ));
    assert_eq!(
        plan_receive(FollowRole::Host, &opened(7051), Some(7), &mut set, "Pat"),
        ReceivePlan::Skip(SkipReason::AlreadyOpenedHere)
    );
}

#[test]
fn a_container_this_player_opened_first_is_not_applied_again() {
    let mut set = OpenedSet::default();
    assert!(set.claim(7051));
    assert_eq!(
        plan_receive(FollowRole::Guest, &opened(7051), Some(7), &mut set, "Pat"),
        ReceivePlan::Skip(SkipReason::AlreadyOpenedHere)
    );
}

#[test]
fn a_replayed_id_removes_the_point_without_a_notice_or_rewards() {
    let mut set = OpenedSet::default();
    let ReceivePlan::Apply {
        items,
        gold,
        notice,
        host_sets,
        ..
    } = plan_receive(
        FollowRole::Guest,
        &remove_only_message(7, 7051),
        Some(7),
        &mut set,
        "Pat",
    )
    else {
        panic!("applies");
    };
    assert!(items.is_empty() && gold == 0 && notice.is_none() && host_sets.is_empty());
}

#[test]
fn the_opened_set_starts_over_in_a_new_zone() {
    let mut set = OpenedSet::default();
    set.enter(Some(7));
    set.claim(7051);
    assert!(!set.enter(Some(7)));
    assert_eq!(set.ids(), vec![7051]);
    assert!(set.enter(Some(8)));
    assert!(set.ids().is_empty());
}

#[test]
fn the_notice_lists_items_and_gold() {
    let text = notice_text("Pat", &opened(7051));
    assert!(text.starts_with("Pat opened a container: "));
    assert!(text.contains("x2"));
    assert!(text.ends_with("150 gold"));
}

fn clearing(id: u16) -> StoryChange {
    StoryChange::Flag {
        array: StoryArray::GameFlags,
        id,
        on: false,
    }
}

fn held_flag() -> ContainerFlag {
    ContainerFlag {
        array: StoryArray::GameFlags,
        id: 1355,
    }
}

#[test]
fn a_held_flag_is_not_cleared_until_the_host_has_it() {
    let mut hold = FlagHold::default();
    hold.hold(&[held_flag()], 0);
    let outcome = hold.filter(vec![clearing(1355), clearing(9)], |_, _| false, 100);
    assert_eq!(outcome.fixes, vec![clearing(9)]);
    assert!(outcome.released.is_empty() && outcome.expired.is_empty());
    let outcome = hold.filter(vec![clearing(1355)], |_, _| true, 200);
    assert_eq!(outcome.released, vec![held_flag()]);
    assert!(hold.is_empty());
}

#[test]
fn a_hold_gives_up_after_three_seconds() {
    let mut hold = FlagHold::default();
    hold.hold(&[held_flag()], 0);
    let outcome = hold.filter(vec![clearing(1355)], |_, _| false, FLAG_HOLD_MS);
    assert_eq!(outcome.expired, vec![held_flag()]);
    assert_eq!(outcome.fixes, vec![clearing(1355)]);
}
