#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetirePlan {
    ReleasePackWhichRemovesActor,
    DespawnThenReleasePack,
    ForgetActorThenReleasePack,
}

pub fn retire_plan(actor_still_ours_to_destroy: bool, pack_guarded: bool) -> RetirePlan {
    match (actor_still_ours_to_destroy, pack_guarded) {
        (true, true) => RetirePlan::ReleasePackWhichRemovesActor,
        (true, false) => RetirePlan::DespawnThenReleasePack,
        (false, _) => RetirePlan::ForgetActorThenReleasePack,
    }
}
