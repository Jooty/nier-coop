use crate::enemies::{EnemyId, EnemySighting};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnforcedDefeatReason {
    NotOnThisCopy,
    IdSharedByTwoEnemiesHere,
    ClassHasItsOwnDamageCode,
    AlreadyDefeatedHere,
    HostHoldsAnotherObjectUnderTheId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyHereForADefeat {
    pub sighting: EnemySighting,
    pub uses_the_base_damage_code: bool,
}

pub fn unforced_defeat_reason(
    here: Option<EnemyHereForADefeat>,
    id_is_shared_by_two_enemies_here: bool,
    host_object_id: Option<u32>,
) -> Option<UnforcedDefeatReason> {
    let Some(here) = here else {
        return Some(UnforcedDefeatReason::NotOnThisCopy);
    };
    if id_is_shared_by_two_enemies_here {
        return Some(UnforcedDefeatReason::IdSharedByTwoEnemiesHere);
    }
    if !here.uses_the_base_damage_code {
        return Some(UnforcedDefeatReason::ClassHasItsOwnDamageCode);
    }
    if here.sighting.is_defeated() {
        return Some(UnforcedDefeatReason::AlreadyDefeatedHere);
    }
    if host_object_id.is_some_and(|object_id| object_id != here.sighting.object_id) {
        return Some(UnforcedDefeatReason::HostHoldsAnotherObjectUnderTheId);
    }
    None
}

pub fn unforced_defeat_line(
    id: EnemyId,
    reason: UnforcedDefeatReason,
    heard_ago_ms: u64,
) -> String {
    let why = match reason {
        UnforcedDefeatReason::NotOnThisCopy => {
            "no enemy with this id is on this copy (it may not be placed here yet)"
        }
        UnforcedDefeatReason::IdSharedByTwoEnemiesHere => {
            "two enemies on this copy share the id, so it is left out of the list"
        }
        UnforcedDefeatReason::ClassHasItsOwnDamageCode => {
            "its class has its own damage code, which the mod does not force"
        }
        UnforcedDefeatReason::AlreadyDefeatedHere => "it is already defeated on this copy",
        UnforcedDefeatReason::HostHoldsAnotherObjectUnderTheId => {
            "the host holds another object under this id"
        }
    };
    format!(
        "enemies: the host's defeat of {} heard {heard_ago_ms} ms ago is not forced on this copy: {why}",
        id.label()
    )
}
