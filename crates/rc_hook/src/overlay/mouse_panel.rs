use std::sync::{Mutex, PoisonError};

use hudhook::imgui::Ui;
use serde_json::{Value, json};

use crate::overlay::click_sources::{ClickSources, SOURCE_NAMES};

static LATEST: Mutex<Option<MouseDiagnostics>> = Mutex::new(None);

#[derive(Debug, Default, Clone)]
pub(super) struct MouseDiagnostics {
    pub(super) os_cursor_screen: Option<[i32; 2]>,
    pub(super) os_cursor_client: Option<[f32; 2]>,
    pub(super) client_size: Option<[f32; 2]>,
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

pub(super) fn publish(mouse: &MouseDiagnostics) {
    *LATEST.lock().unwrap_or_else(PoisonError::into_inner) = Some(mouse.clone());
}

pub fn latest_mouse_json() -> Value {
    let Some(mouse) = LATEST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
    else {
        return json!({"ok": false, "error": "overlay has not drawn a frame yet"});
    };
    json!({
        "ok": true,
        "os_cursor_screen": mouse.os_cursor_screen,
        "os_cursor_client": mouse.os_cursor_client,
        "os_cursor_visible": mouse.os_cursor_visible,
        "client_size": mouse.client_size,
        "display_size": mouse.display_size,
        "imgui_position": mouse.imgui_position,
        "imgui_buttons": mouse.imgui_buttons,
        "want_capture_mouse": mouse.want_capture_mouse,
        "cursor_mode": mouse.cursor_mode,
        "game_mouse_acquired": mouse.game_mouse_acquired,
        "blocked_set_cursor_pos": mouse.blocked_set_cursor_pos,
        "blocked_clip_cursor": mouse.blocked_clip_cursor,
        "blocked_acquire": mouse.blocked_acquire,
        "spoofed_get_cursor_pos": mouse.spoofed_get_cursor_pos,
        "click_sources": click_sources_json(&mouse.clicks),
    })
}

pub(super) fn draw_mouse_diagnostics(ui: &Ui, mouse: &MouseDiagnostics) {
    ui.text(format!(
        "os cursor: screen {:?} client {:?} visible {}",
        mouse.os_cursor_screen, mouse.os_cursor_client, mouse.os_cursor_visible
    ));
    ui.text(format!(
        "client size {:?} | display size {:?}",
        mouse.client_size, mouse.display_size
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

fn click_sources_json(clicks: &ClickSources) -> Value {
    let sources = SOURCE_NAMES.iter().enumerate().map(|(index, name)| {
        (
            (*name).to_owned(),
            json!({
                "left": clicks.down[index][0],
                "right": clicks.down[index][1],
                "down_events": clicks.down_events[index],
            }),
        )
    });
    Value::Object(sources.collect())
}
