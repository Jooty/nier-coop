#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapGeometry {
    pub map_width: f32,
    pub map_height: f32,
    pub half_viewport_width: f32,
    pub half_viewport_height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconPlacement {
    pub x: f32,
    pub y: f32,
    pub clamped_to_edge: bool,
}

pub fn partner_icon_placement(
    player_map: [f32; 2],
    partner_map: [f32; 2],
    geometry: &MapGeometry,
) -> Option<IconPlacement> {
    let finite = player_map
        .iter()
        .chain(partner_map.iter())
        .chain([
            &geometry.map_width,
            &geometry.map_height,
            &geometry.half_viewport_width,
            &geometry.half_viewport_height,
            &geometry.offset_x,
            &geometry.offset_y,
        ])
        .all(|value| value.is_finite());
    if !finite || geometry.half_viewport_width <= 0.0 || geometry.half_viewport_height <= 0.0 {
        return None;
    }
    let delta_x = (partner_map[0] - player_map[0]) * geometry.map_width;
    let delta_y = (partner_map[1] - player_map[1]) * geometry.map_height;
    let kept_x = delta_x.clamp(-geometry.half_viewport_width, geometry.half_viewport_width);
    let kept_y = delta_y.clamp(
        -geometry.half_viewport_height,
        geometry.half_viewport_height,
    );
    Some(IconPlacement {
        x: player_map[0] * geometry.map_width + geometry.offset_x + kept_x,
        y: player_map[1] * geometry.map_height + geometry.offset_y + kept_y,
        clamped_to_edge: kept_x != delta_x || kept_y != delta_y,
    })
}

pub fn minimap_is_shown(
    shown_flag: u8,
    minimap_enabled_byte: u8,
    area: u32,
    area_count: u32,
) -> bool {
    shown_flag != 0 && minimap_enabled_byte != 0 && area < area_count
}
