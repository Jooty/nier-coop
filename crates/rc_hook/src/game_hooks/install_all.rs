use crate::build_gate::VerifiedBuild;
use crate::game_hooks::{invincibility, mouse_block, noclip};

pub fn install(build: VerifiedBuild) {
    invincibility::install(build);
    noclip::install(build);
    mouse_block::install();
}
