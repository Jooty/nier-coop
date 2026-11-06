use crate::boot;
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::{
    cursor_calls, invincibility, mouse_block, noclip, window_focus, zone_gate,
};
use crate::net::AutoConfig;
use crate::saves;

pub fn install(build: VerifiedBuild) {
    invincibility::install(build);
    noclip::install(build);
    zone_gate::install(build);
    crate::render_camera::install(build);
    cursor_calls::install();
    mouse_block::install();
    match AutoConfig::load_beside_game() {
        Ok(Some(config)) => {
            if config.link.is_some() {
                saves::install_coop_files(build, !config.block_saves);
            }
            if config.block_saves {
                saves::install(build);
            }
            if config.keep_running {
                window_focus::install(build);
            }
            if let Some(slot) = config.boot_slot {
                boot::install(build, slot);
            }
        }
        Ok(None) => {}
        Err(error) => log_line(&format!("hooks: {error}")),
    }
}
