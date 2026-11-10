use rc_game::stay_together::{TELEPORT_GIVE_UP_MS, TeleportCheck, check_teleport};

const TARGET: [f32; 3] = [-27.3, 126.1, 24.0];

#[test]
fn standing_on_the_target_has_arrived() {
    assert_eq!(check_teleport(TARGET, TARGET, 0), TeleportCheck::Arrived);
}

#[test]
fn a_little_off_the_target_still_counts() {
    let near = [TARGET[0] + 0.5, TARGET[1], TARGET[2] - 0.5];
    assert_eq!(check_teleport(TARGET, near, 100), TeleportCheck::Arrived);
}

#[test]
fn far_away_waits_until_the_deadline() {
    let far = [0.0, 0.0, 0.0];
    assert_eq!(check_teleport(TARGET, far, 100), TeleportCheck::Waiting);
    assert_eq!(
        check_teleport(TARGET, far, TELEPORT_GIVE_UP_MS),
        TeleportCheck::Missed
    );
}

#[test]
fn arriving_at_the_deadline_still_counts() {
    assert_eq!(
        check_teleport(TARGET, TARGET, TELEPORT_GIVE_UP_MS + 1),
        TeleportCheck::Arrived
    );
}

#[test]
fn an_unreadable_position_never_arrives() {
    let broken = [f32::NAN, 0.0, 0.0];
    assert_eq!(check_teleport(TARGET, broken, 0), TeleportCheck::Waiting);
}
