#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapTuning {
    pub enabled: bool,
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
    pub world_range: f32,
    pub dot_size: f32,
    pub rotate_with_camera: bool,
}

impl MinimapTuning {
    pub const fn new() -> Self {
        Self {
            enabled: true,
            center_x: 1790.0,
            center_y: 160.0,
            radius: 110.0,
            world_range: 120.0,
            dot_size: 9.0,
            rotate_with_camera: false,
        }
    }
}

impl Default for MinimapTuning {
    fn default() -> Self {
        Self::new()
    }
}
