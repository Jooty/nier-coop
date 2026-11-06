use crate::build_gate::VerifiedBuild;
use crate::enemies::{drop_hook, hit_hook, hold_hook, word_drop_hook};

pub fn install(build: VerifiedBuild) {
    hit_hook::install(build);
    drop_hook::install(build);
    hold_hook::install(build);
    word_drop_hook::install(build);
}
