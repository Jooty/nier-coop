use crate::enemies::LARGEST_STORY_ACTOR_MAX_HP;

/// Whether the guest destroys its copy when the host's game removes that enemy. A story actor
/// (a 1 HP prop or villager) or one running its own talk or script is left to the guest's own
/// game: its scripts may wait on it, and the host may have removed an older actor of the same id.
pub fn host_removal_destroys_copy(max_hp: Option<i32>, runs_own_event: bool) -> bool {
    !runs_own_event && max_hp.is_some_and(|max_hp| max_hp > LARGEST_STORY_ACTOR_MAX_HP)
}
