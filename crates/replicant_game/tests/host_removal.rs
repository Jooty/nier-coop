use replicant_game::enemies::host_removal_destroys_copy;

#[test]
fn a_fighting_enemy_removed_by_the_host_is_removed_here_too() {
    assert!(host_removal_destroys_copy(Some(2000), false));
}

#[test]
fn a_one_hp_story_actor_is_left_to_the_guests_own_script() {
    assert!(!host_removal_destroys_copy(Some(1), false));
}

#[test]
fn an_actor_running_its_own_talk_or_script_is_left_alone() {
    assert!(!host_removal_destroys_copy(Some(2000), true));
}

#[test]
fn an_actor_whose_hp_cannot_be_read_is_left_alone() {
    assert!(!host_removal_destroys_copy(None, false));
}
