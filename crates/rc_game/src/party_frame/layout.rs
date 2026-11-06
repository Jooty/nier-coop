use crate::party_frame::{
    BASE_FRAME_BOTTOM_SPRITE_PIXELS, BASE_FRAME_TOP_SPRITE_PIXELS, FONT_CELL_UNITS,
    PARTY_FRAME_BAR_RIGHT_FROM_SCREEN_RIGHT_REFERENCE_PIXELS,
    PARTY_FRAME_NAME_GAP_REFERENCE_PIXELS, PARTY_FRAME_RIGHT_MARGIN_REFERENCE_PIXELS,
    PARTY_FRAME_STATUS_INSET_REFERENCE_PIXELS, PARTY_FRAME_TOP_REFERENCE_PIXELS, PartyFrameTuning,
    REFERENCE_SCREEN_HEIGHT,
};

/// Screen positions of the party frame. The two anchors are the right end of each bar's well and
/// the top of its black frame; the bars grow to the left from there, like the game's own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartyFrameLayout {
    pub life_anchor: [f32; 2],
    pub magic_anchor: [f32; 2],
    pub bar_scale: f32,
    pub name_origin: [f32; 2],
    pub own_name_origin: [f32; 2],
    pub name_scale: f32,
    pub name_max_width: f32,
    pub status_scale: f32,
    pub status_origin_y: f32,
    pub status_right_edge: f32,
    pub scale: f32,
}

pub fn party_frame_layout(
    screen_width: f32,
    screen_height: f32,
    tuning: &PartyFrameTuning,
) -> PartyFrameLayout {
    let scale = screen_height / REFERENCE_SCREEN_HEIGHT;
    let bar_scale = scale * tuning.bar_scale();
    let frame_height = (BASE_FRAME_BOTTOM_SPRITE_PIXELS - BASE_FRAME_TOP_SPRITE_PIXELS) * bar_scale;
    let anchor_x = screen_width
        - (PARTY_FRAME_BAR_RIGHT_FROM_SCREEN_RIGHT_REFERENCE_PIXELS - tuning.offset_x()) * scale;
    let life_top = (PARTY_FRAME_TOP_REFERENCE_PIXELS + tuning.offset_y()) * scale;
    let magic_top = life_top + frame_height + tuning.bar_gap() * scale;
    let name_size = tuning.name_size() * scale;
    let status_size = tuning.status_size() * scale;
    let name_left =
        anchor_x + (PARTY_FRAME_NAME_GAP_REFERENCE_PIXELS + tuning.name_offset_x()) * scale;
    let name_y = life_top + (frame_height - name_size) * 0.5 + tuning.name_offset_y() * scale;
    let right = screen_width - PARTY_FRAME_RIGHT_MARGIN_REFERENCE_PIXELS * scale;
    PartyFrameLayout {
        life_anchor: [anchor_x, life_top],
        magic_anchor: [anchor_x, magic_top],
        bar_scale,
        name_origin: [name_left, name_y],
        own_name_origin: [name_left, name_y - tuning.name_row_gap() * scale],
        name_scale: name_size / FONT_CELL_UNITS,
        name_max_width: (right - name_left).max(name_size * 3.0),
        status_scale: status_size / FONT_CELL_UNITS,
        status_origin_y: life_top
            + (frame_height - status_size) * 0.5
            + tuning.status_offset_y() * scale,
        status_right_edge: anchor_x
            + (tuning.status_offset_x() - PARTY_FRAME_STATUS_INSET_REFERENCE_PIXELS) * scale,
        scale,
    }
}
