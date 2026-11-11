use nc_hook::actions::{ActionQueue, QueueFull};

#[derive(Debug, Clone, PartialEq, Eq)]
enum MadeUpAction {
    Wave,
    Jump(u8),
}

#[test]
fn actions_come_out_in_the_order_they_went_in() {
    let queue = ActionQueue::new(4);
    queue.enqueue(MadeUpAction::Wave).expect("room");
    queue.enqueue(MadeUpAction::Jump(2)).expect("room");
    assert_eq!(
        queue.take_all(),
        vec![MadeUpAction::Wave, MadeUpAction::Jump(2)]
    );
}

#[test]
fn taking_everything_empties_the_queue() {
    let queue = ActionQueue::new(4);
    queue.enqueue(MadeUpAction::Wave).expect("room");
    assert_eq!(queue.take_all().len(), 1);
    assert!(queue.take_all().is_empty());
}

#[test]
fn a_full_queue_refuses_the_next_action_and_keeps_the_rest() {
    let queue = ActionQueue::new(2);
    queue.enqueue(MadeUpAction::Jump(1)).expect("room");
    queue.enqueue(MadeUpAction::Jump(2)).expect("room");
    assert_eq!(
        queue.enqueue(MadeUpAction::Jump(3)),
        Err(QueueFull { capacity: 2 })
    );
    assert_eq!(queue.take_all().len(), 2);
}

#[test]
fn the_refusal_names_the_capacity() {
    assert_eq!(
        QueueFull { capacity: 16 }.to_string(),
        "action queue holds 16 already"
    );
}

#[test]
fn room_is_free_again_after_a_take() {
    let queue = ActionQueue::new(1);
    queue.enqueue(MadeUpAction::Wave).expect("room");
    assert!(queue.enqueue(MadeUpAction::Wave).is_err());
    let _ = queue.take_all();
    assert!(queue.enqueue(MadeUpAction::Wave).is_ok());
}
