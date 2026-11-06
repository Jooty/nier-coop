use crate::boot;
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::{
    cursor_calls, door_gate, focus_menu_block, invincibility, menu_no_pause, mouse_block, noclip,
    window_focus, zone_gate,
};
use crate::net::AutoConfig;
use crate::player_names;
use crate::saves;

pub fn install(build: VerifiedBuild) {
    invincibility::install(build);
    noclip::install(build);
    zone_gate::install(build);
    menu_no_pause::install(build);
    door_gate::install(build);
    crate::render_camera::install(build);
    cursor_calls::install();
    mouse_block::install();
    player_names::install(build);
    match AutoConfig::load_beside_game() {
        Ok(Some(config)) => {
            if config.link.is_some() {
                saves::install_coop_files(build, !config.block_saves);
                crate::effects::install(build);
                crate::npcs::install(build);
                crate::music::install(build);
                crate::minimap::install(build);
                crate::pickups::install(build);
                crate::containers::install(build);
                crate::dialogue::install(build);
                crate::story_writes::install(build);
                crate::story_relay::install(build);
                crate::script_rolls::install(build);
                crate::boss_messages::install(build);
                crate::rewind::install(build);
                crate::fade_sync::install(build);
                crate::cutscene_skip::install(build);
                crate::enemies::install(build);
                crate::player_body::install(build, config.body.unwrap_or_default());
            }
            if config.block_saves {
                saves::install(build);
            }
            if config.keep_running || config.link.is_some() {
                window_focus::install(build, config.keep_running);
            }
            if config.dev_no_focus_menu {
                focus_menu_block::install(build);
            }
            if let Some(slot) = config.boot_slot {
                boot::install(build, slot);
            }
            if let Some(slot) = config.new_game_slot {
                let name = config.new_game_name.as_deref().or(config.name.as_deref());
                boot::install_new_game(build, slot, name, config.link.is_some());
            }
        }
        Ok(None) => {}
        Err(error) => log_line(&format!("hooks: {error}")),
    }
}
