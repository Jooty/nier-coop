use replicant_game::door_sync::{
    DoorRegistry, GUEST_USE_WINDOW_MS, GuestUseWindow, PendingDoorStates,
    SNAPSHOT_APPLY_GIVE_UP_MS, SNAPSHOT_SETTLE_MS, SnapshotSchedule, action_of_message,
    changes_for_resulting_state, door_reached_state, message_of_action, plan_door_state,
    sender_phase_matches_local,
};
use replicant_proto::{DoorAction, DoorChange, DoorState};

fn state(layout_id: u16, open: bool, locked: bool) -> DoorState {
    DoorState {
        layout_id,
        open,
        locked,
    }
}

#[test]
fn only_the_four_state_messages_are_mirrored() {
    for message in [0x2A, 0x2C, 0x2D, 0x2E] {
        let action = action_of_message(message).expect("a mirrored message");
        assert_eq!(message_of_action(action), message);
    }
    for message in [0x11, 0x17, 0x2B, 0x2F, 0] {
        assert_eq!(action_of_message(message), None);
    }
}

#[test]
fn the_registry_counts_each_door_once() {
    let mut registry = DoorRegistry::default();
    assert!(registry.note(602));
    assert!(registry.note(601));
    assert!(!registry.note(602));
    assert_eq!(registry.ids(), vec![601, 602]);
    registry.clear();
    assert!(registry.ids().is_empty());
}

#[test]
fn a_guest_use_counts_only_for_that_door_inside_the_window() {
    let mut uses = GuestUseWindow::default();
    uses.note_use(601, 1000);
    assert!(uses.was_used_recently(601, 1000 + GUEST_USE_WINDOW_MS - 1));
    assert!(!uses.was_used_recently(601, 1000 + GUEST_USE_WINDOW_MS));
    assert!(!uses.was_used_recently(602, 1500));
}

#[test]
fn the_host_sends_once_after_both_players_settle_in_one_zone() {
    let mut schedule = SnapshotSchedule::new();
    schedule.observe(Some(4), Some(5), 0);
    assert_eq!(schedule.take_due(SNAPSHOT_SETTLE_MS * 2), None);
    schedule.observe(Some(4), Some(4), 100);
    assert_eq!(schedule.take_due(100 + SNAPSHOT_SETTLE_MS - 1), None);
    assert_eq!(schedule.take_due(100 + SNAPSHOT_SETTLE_MS), Some(4));
    assert_eq!(schedule.take_due(100 + SNAPSHOT_SETTLE_MS * 2), None);
}

#[test]
fn a_new_zone_for_both_schedules_a_new_send() {
    let mut schedule = SnapshotSchedule::new();
    schedule.observe(Some(4), Some(4), 0);
    assert_eq!(schedule.take_due(SNAPSHOT_SETTLE_MS), Some(4));
    schedule.observe(Some(7), Some(4), 10_000);
    schedule.observe(Some(7), Some(7), 10_500);
    assert_eq!(schedule.take_due(10_500 + SNAPSHOT_SETTLE_MS), Some(7));
}

#[test]
fn a_door_plan_only_changes_what_differs() {
    assert!(plan_door_state(true, true, &state(1, true, true)).is_empty());
    assert_eq!(
        plan_door_state(false, false, &state(1, true, true)),
        vec![(DoorAction::Open, true), (DoorAction::Lock, true)]
    );
    assert_eq!(
        plan_door_state(true, true, &state(1, false, false)),
        vec![(DoorAction::Lock, false), (DoorAction::Close, true)]
    );
}

#[test]
fn doors_not_loaded_yet_are_retried_then_given_up() {
    let mut pending = PendingDoorStates::default();
    pending.receive(9, &[state(601, true, false), state(602, true, true)], 0);
    let wrong_zone = pending.retry(Some(3), 600, |_| true);
    assert_eq!(wrong_zone.applied, 0);
    let first = pending.retry(Some(9), 1200, |door| door.layout_id == 601);
    assert_eq!((first.applied, first.given_up), (1, 0));
    let late = pending.retry(Some(9), SNAPSHOT_APPLY_GIVE_UP_MS, |_| false);
    assert_eq!((late.applied, late.given_up), (0, 1));
    assert!(pending.is_empty());
}

#[test]
fn a_locked_door_is_unlocked_before_it_opens_and_locked_after_it_closes() {
    assert_eq!(
        plan_door_state(false, true, &state(1, true, false)),
        vec![(DoorAction::Lock, false), (DoorAction::Open, true)]
    );
    assert_eq!(
        plan_door_state(false, true, &state(1, true, true)),
        vec![
            (DoorAction::Lock, false),
            (DoorAction::Open, true),
            (DoorAction::Lock, true)
        ]
    );
    assert_eq!(
        plan_door_state(true, false, &state(1, false, true)),
        vec![(DoorAction::Close, true), (DoorAction::Lock, true)]
    );
}

#[test]
fn a_door_that_missed_the_wanted_state_is_not_reported_done() {
    let wanted = state(1, true, false);
    assert!(door_reached_state(true, false, &wanted));
    assert!(!door_reached_state(false, false, &wanted));
    assert!(!door_reached_state(true, true, &wanted));
}

#[test]
fn a_door_message_counts_only_in_the_senders_zone() {
    assert!(sender_phase_matches_local(7, Some(7)));
    assert!(!sender_phase_matches_local(7, Some(8)));
    assert!(!sender_phase_matches_local(7, None));
}

#[test]
fn the_host_echo_of_a_refused_open_says_closed() {
    let changes = changes_for_resulting_state(7, &state(601, false, true));
    assert_eq!(
        changes,
        vec![
            DoorChange {
                layout_id: 601,
                action: DoorAction::Close,
                flag: true,
                phase: 7
            },
            DoorChange {
                layout_id: 601,
                action: DoorAction::Lock,
                flag: true,
                phase: 7
            },
        ]
    );
    let open = changes_for_resulting_state(7, &state(601, true, false));
    assert_eq!(open[0].action, DoorAction::Lock);
    assert!(!open[0].flag);
    assert_eq!(open[1].action, DoorAction::Open);
}
