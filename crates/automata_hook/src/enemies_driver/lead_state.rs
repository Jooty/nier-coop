use automata_game::enemies::EnemyId;
use automata_game::enemy_lead::{LeadFollower, PlaceLog, StartTally};

/// One shared enemy as the last count found it on this copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LedEnemy {
    pub id: EnemyId,
    pub body: usize,
    pub handle: u32,
}

/// The host-led part of the enemies driver: what the host counts and sends, what the guest
/// heard and follows each frame, and the place log both keep.
#[derive(Debug, Default)]
pub(super) struct LeadState {
    pub active: bool,
    pub states_are_hooked: bool,
    pub targets_are_hooked: bool,
    pub tally: StartTally,
    pub follower: LeadFollower,
    pub enemies: Vec<LedEnemy>,
    pub place_log: PlaceLog,
    pub first_pose_heard: bool,
    pub first_start_played: bool,
    pub first_place_written: bool,
    pub first_target_written: bool,
}
