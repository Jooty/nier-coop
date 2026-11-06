use hudhook::imgui::Ui;

use crate::overlay::click_sources::{ClickSources, SOURCE_NAMES};

#[derive(Debug, Default, Clone)]
pub(super) struct MouseDiagnostics {
    pub(super) os_cursor_screen: Option<[i32; 2]>,
    pub(super) os_cursor_client: Option<[f32; 2]>,
    pub(super) client_size: Option<[f32; 2]>,
    pub(super) game_window: usize,
    pub(super) cursor_key_vk_seen: u32,
    pub(super) cursor_mode_latched: bool,
    pub(super) display_size: [f32; 2],
    pub(super) os_cursor_visible: bool,
    pub(super) imgui_position: [f32; 2],
    pub(super) imgui_buttons: [bool; 2],
    pub(super) want_capture_mouse: bool,
    pub(super) cursor_mode: bool,
    pub(super) game_mouse_acquired: bool,
    pub(super) blocked_set_cursor_pos: u32,
    pub(super) blocked_clip_cursor: u32,
    pub(super) blocked_acquire: u32,
    pub(super) spoofed_get_cursor_pos: u32,
    pub(super) clicks: ClickSources,
}

pub(super) fn draw_mouse_diagnostics(ui: &Ui, mouse: &MouseDiagnostics) {
    ui.text(format!(
        "os cursor: screen {:?} client {:?} visible {}",
        mouse.os_cursor_screen, mouse.os_cursor_client, mouse.os_cursor_visible
    ));
    ui.text(format!(
        "window {:#x} | client size {:?} | display size {:?}",
        mouse.game_window, mouse.client_size, mouse.display_size
    ));
    ui.text(format!(
        "cursor key VK {:#x} | latched {}",
        mouse.cursor_key_vk_seen, mouse.cursor_mode_latched
    ));
    ui.text(format!(
        "imgui: pos {:.0} {:.0} left {} right {} want capture {}",
        mouse.imgui_position[0],
        mouse.imgui_position[1],
        mouse.imgui_buttons[0],
        mouse.imgui_buttons[1],
        mouse.want_capture_mouse
    ));
    ui.text(format!(
        "cursor mode {} | game mouse acquired {}",
        mouse.cursor_mode, mouse.game_mouse_acquired
    ));
    ui.text(format!(
        "blocked: SetCursorPos {} ClipCursor {} Acquire {} GetCursorPos {}",
        mouse.blocked_set_cursor_pos,
        mouse.blocked_clip_cursor,
        mouse.blocked_acquire,
        mouse.spoofed_get_cursor_pos
    ));
    for (index, name) in SOURCE_NAMES.iter().enumerate() {
        ui.text(format!(
            "click {name}: left {} right {} downs {}",
            mouse.clicks.down[index][0],
            mouse.clicks.down[index][1],
            mouse.clicks.down_events[index]
        ));
    }
}
