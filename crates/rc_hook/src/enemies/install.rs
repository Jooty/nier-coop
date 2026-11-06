use crate::build_gate::VerifiedBuild;
use crate::enemies::{drop_hook, hit_hook, hold_hook};

pub fn install(build: VerifiedBuild) {
    hit_hook::install(build);
    drop_hook::install(build);
    hold_hook::install(build);
}
