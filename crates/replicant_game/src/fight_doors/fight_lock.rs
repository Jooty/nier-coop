use crate::PHASE_TABLE;
use crate::fight_doors::{FIGHT_DOORS, INJECTED_RUN_LOCK_WINDOW_MS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockOrigin {
    OwnRun,
    InjectedRun,
    MirroredFromHost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldReason {
    InjectedRun,
    MirroredFromHost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FightLockVerdict {
    Apply,
    Hold(HoldReason),
}

impl LockOrigin {
    pub fn of_a_local_lock(ms_since_an_injected_run_started: Option<u64>) -> Self {
        match ms_since_an_injected_run_started {
            Some(age) if age < INJECTED_RUN_LOCK_WINDOW_MS => Self::InjectedRun,
            _ => Self::OwnRun,
        }
    }
}

impl HoldReason {
    pub fn text(self) -> &'static str {
        match self {
            Self::InjectedRun => "the run was started by the partner's trigger",
            Self::MirroredFromHost => "the host's lock; this player's own trap locks its own fight",
        }
    }
}

pub fn is_fight_door(phase: &str, door_id: i32) -> bool {
    FIGHT_DOORS
        .iter()
        .any(|&(zone, door)| zone == phase && door == door_id)
}

pub fn fight_door_in_zone(phase_index: u16, door_id: u16) -> bool {
    PHASE_TABLE
        .get(usize::from(phase_index))
        .is_some_and(|entry| is_fight_door(entry.name, i32::from(door_id)))
}

pub fn judge_door_lock(
    phase: &str,
    door_id: i32,
    locking: bool,
    origin: LockOrigin,
) -> FightLockVerdict {
    if !locking || !is_fight_door(phase, door_id) {
        return FightLockVerdict::Apply;
    }
    match origin {
        LockOrigin::OwnRun => FightLockVerdict::Apply,
        LockOrigin::InjectedRun => FightLockVerdict::Hold(HoldReason::InjectedRun),
        LockOrigin::MirroredFromHost => FightLockVerdict::Hold(HoldReason::MirroredFromHost),
    }
}
