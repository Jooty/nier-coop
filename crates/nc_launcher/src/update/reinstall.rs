use nc_launcher_core::GameProfile;

use crate::update::UpdatePlan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReinstallDecision {
    InstallFromThisUpdate,
    ManifestHasNoHookForThisGame,
}

pub fn reinstall_decision(plan: &UpdatePlan, profile: &GameProfile) -> ReinstallDecision {
    if plan.hook_for(profile.game_id).is_some() {
        ReinstallDecision::InstallFromThisUpdate
    } else {
        ReinstallDecision::ManifestHasNoHookForThisGame
    }
}

pub fn untouched_install_line(profile: &GameProfile) -> String {
    format!(
        "this build has no hook for {}; its installed mod was left as it is",
        profile.display_name
    )
}
