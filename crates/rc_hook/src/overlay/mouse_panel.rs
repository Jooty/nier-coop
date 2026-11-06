use hudhook::imgui::{TreeNodeFlags, Ui};

#[derive(Debug, Default)]
pub(super) struct MouseDiagnostics {
    pub(super) os_cursor_screen: Option<[i32; 2]>,
    pub(super) os_cursor_client: Option<[f32; 2]>,
    pub(super) os_cursor_visible: bool,
    pub(super) imgui_position: [f32; 2],
    pub(super) imgui_buttons: [bool; 2],
    pub(super) want_capture_mouse: bool,
    pub(super) cursor_mode: bool,
    pub(super) game_mouse_acquired: bool,
    pub(super) blocked_set_cursor_pos: u32,
    pub(super) blocked_clip_cursor: u32,
    pub(super) blocked_acquire: u32,
}

pub(super) fn draw_mouse_diagnostics(ui: &Ui, mouse: &MouseDiagnostics) {
    if !ui.collapsing_header("mouse", TreeNodeFlags::empty()) {
        return;
    }
    ui.text(format!(
        "os cursor: screen {:?} client {:?} visible {}",
        mouse.os_cursor_screen, mouse.os_cursor_client, mouse.os_cursor_visible
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
        "blocked: SetCursorPos {} ClipCursor {} Acquire {}",
        mouse.blocked_set_cursor_pos, mouse.blocked_clip_cursor, mouse.blocked_acquire
    ));
}
