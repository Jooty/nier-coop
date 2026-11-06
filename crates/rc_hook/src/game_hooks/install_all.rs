use crate::build_gate::VerifiedBuild;
use crate::game_hooks::{cursor_calls, invincibility, mouse_block, noclip};

pub fn install(build: VerifiedBuild) {
    invincibility::install(build);
    noclip::install(build);
    cursor_calls::install();
    mouse_block::install();
}
