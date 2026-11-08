mod cursor_calls;
mod door_gate;
mod door_walk;
mod focus_menu_block;
mod hook_slot;
mod install_all;
mod invincibility;
mod menu_free_calls;
mod menu_no_pause;
mod mouse_block;
mod mouse_buttons;
mod noclip;
mod noclip_keys;
mod stuck_menu_mask;
mod teleport;
mod window_focus;
mod zone_gate;

pub use cursor_calls::{
    blocked_clip_cursor_calls, blocked_set_cursor_pos_calls, clear_cursor_clip,
    real_cursor_position, spoofed_get_cursor_pos_calls,
};
pub(crate) use hook_slot::hook_address;
pub use install_all::install;
pub use invincibility::{
    is_enabled as invincibility_enabled, is_installed as invincibility_installed,
    set_enabled as set_invincible, set_local_health_through_game, take_lethal_hit_seen,
};
pub use menu_no_pause::{set_downed_controls_locked, update_menu_no_pause};
pub use mouse_block::{
    blocked_acquire_calls, game_mouse_acquired, set_cursor_mode, set_game_mouse_blocked,
};
pub use mouse_buttons::{direct_input_buttons, direct_input_down_events};
pub use noclip::{
    is_enabled as noclip_enabled, is_installed as noclip_installed, set_enabled as set_noclip,
};
pub use noclip_keys::{game_window_is_foreground, set_keyboard_taken_by_overlay};
pub use teleport::{request_teleport, teleport_available};
pub use window_focus::{
    keep_running_enabled, set_keep_running, take_focus_event_counts, window_unfocused,
};
