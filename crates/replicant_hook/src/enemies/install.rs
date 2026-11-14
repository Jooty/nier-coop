use crate::build_gate::VerifiedBuild;
#[cfg(feature = "probes")]
use crate::enemies::boss_bar_probe;
use crate::enemies::{bullet_hit_hook, drop_hook, hit_hook, hold_hook, spawn_hook, word_drop_hook};

pub fn install(build: VerifiedBuild) {
    #[cfg(feature = "probes")]
    boss_bar_probe::install(build);
    hit_hook::install(build);
    bullet_hit_hook::install(build);
    drop_hook::install(build);
    hold_hook::install(build);
    word_drop_hook::install(build);
    spawn_hook::install(build);
}
