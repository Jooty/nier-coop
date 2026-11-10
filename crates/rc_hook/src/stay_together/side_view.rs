use rc_game::GameMemory;
use rc_game::offsets::SIDE_VIEW_FLAG_RVA;

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;

pub(super) fn side_view_on(build: &VerifiedBuild) -> Option<bool> {
    let address = build.module_base() + u64::from(SIDE_VIEW_FLAG_RVA);
    let flag = InProcessMemory::new(build).read_u8(address).ok()?;
    Some(flag != 0)
}

pub(crate) fn side_view_now(build: &VerifiedBuild) -> bool {
    side_view_on(build).unwrap_or(false)
}
