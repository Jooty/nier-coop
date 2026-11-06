use crate::party_frame::{
    FONT_CELL_UNITS, GAUGE_LINE_RECT, GAUGE_TRACK_RECT, PARTY_FRAME_GAUGE_WIDTH_REFERENCE_PIXELS,
    PARTY_FRAME_ICON_GAP_REFERENCE_PIXELS, PARTY_FRAME_ICON_REFERENCE_PIXELS,
    PARTY_FRAME_NAME_ROW_HEIGHT_REFERENCE_PIXELS, PARTY_FRAME_NAME_SIZE_REFERENCE_PIXELS,
    PARTY_FRAME_RIGHT_MARGIN_REFERENCE_PIXELS, PARTY_FRAME_TOP_REFERENCE_PIXELS,
    REFERENCE_SCREEN_HEIGHT,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartyFrameLayout {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub icon_center: [f32; 2],
    pub icon_radius: f32,
    pub name_origin: [f32; 2],
    pub name_scale: f32,
    pub name_max_width: f32,
    pub status_right_edge: f32,
    pub gauge_origin: [f32; 2],
    pub gauge_scale: [f32; 2],
    pub scale: f32,
}

pub fn party_frame_layout(screen_width: f32, screen_height: f32) -> PartyFrameLayout {
    let scale = screen_height / REFERENCE_SCREEN_HEIGHT;
    let width = PARTY_FRAME_GAUGE_WIDTH_REFERENCE_PIXELS * scale;
    let gauge_scale = [width / GAUGE_LINE_RECT.width() as f32, scale];
    let gauge_height = (GAUGE_LINE_RECT.bottom - GAUGE_TRACK_RECT.top) as f32 * scale;
    let name_row = PARTY_FRAME_NAME_ROW_HEIGHT_REFERENCE_PIXELS * scale;
    let icon = PARTY_FRAME_ICON_REFERENCE_PIXELS * scale;
    let icon_gap = PARTY_FRAME_ICON_GAP_REFERENCE_PIXELS * scale;
    let right = screen_width - PARTY_FRAME_RIGHT_MARGIN_REFERENCE_PIXELS * scale;
    let min = [right - width, PARTY_FRAME_TOP_REFERENCE_PIXELS * scale];
    let max = [right, min[1] + name_row + gauge_height];
    let name_left = min[0] + icon + icon_gap;
    PartyFrameLayout {
        min,
        max,
        icon_center: [min[0] + icon * 0.5, min[1] + name_row * 0.5],
        icon_radius: icon * 0.5,
        name_origin: [name_left, min[1]],
        name_scale: PARTY_FRAME_NAME_SIZE_REFERENCE_PIXELS * scale / FONT_CELL_UNITS,
        name_max_width: right - name_left,
        status_right_edge: right,
        gauge_origin: [min[0], min[1] + name_row],
        gauge_scale,
        scale,
    }
}
