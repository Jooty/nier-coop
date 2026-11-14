use crate::partner_hud::{MATRIX_FLOATS, NEAREST_TAG_DEPTH_UNITS, TAG_EDGE_MARGIN_FRACTION};

/// The game camera's two matrices as its own world-to-screen reads them: row vectors, the view's
/// translation in the last row, the camera looking down minus z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TagCamera {
    pub view: [f32; MATRIX_FLOATS],
    pub projection: [f32; MATRIX_FLOATS],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projected {
    OnScreen { x: f32, y: f32 },
    EdgeArrow { x: f32, y: f32, angle_radians: f32 },
    Invalid,
}

impl TagCamera {
    /// The point in camera space: x to the right, y up, z towards the viewer.
    pub fn to_view(&self, point: [f32; 3]) -> [f32; 3] {
        let view = &self.view;
        let axis = |column: usize| {
            point[0] * view[column]
                + point[1] * view[4 + column]
                + point[2] * view[8 + column]
                + view[12 + column]
        };
        let w = axis(3);
        [axis(0) / w, axis(1) / w, axis(2) / w]
    }

    /// The unit directions of the camera's right and forward on the ground plane (x, z).
    pub fn ground_axes(&self) -> Option<([f32; 2], [f32; 2])> {
        let right = unit([self.view[0], self.view[8]])?;
        let forward = unit([-self.view[2], -self.view[10]])?;
        Some((right, forward))
    }
}

/// Where a world point is on a screen of the given size, the way the game's own world-to-screen
/// places it; a point off the screen or behind the camera gets an arrow at the screen edge.
pub fn project_tag(camera: &TagCamera, point: [f32; 3], width: f32, height: f32) -> Projected {
    let usable = width > 0.0
        && height > 0.0
        && width.is_finite()
        && height.is_finite()
        && point.iter().all(|axis| axis.is_finite());
    if !usable {
        return Projected::Invalid;
    }
    let in_view = camera.to_view(point);
    let projection = &camera.projection;
    let clip = |column: usize| {
        in_view[0] * projection[column]
            + in_view[1] * projection[4 + column]
            + in_view[2] * projection[8 + column]
            + projection[12 + column]
    };
    let (lateral_x, lateral_y, depth) = (clip(0), clip(1), -in_view[2]);
    if !(lateral_x.is_finite() && lateral_y.is_finite() && depth.is_finite()) {
        return Projected::Invalid;
    }
    if depth > NEAREST_TAG_DEPTH_UNITS {
        let ndc = (lateral_x / depth, lateral_y / depth);
        if ndc.0.abs() <= 1.0 && ndc.1.abs() <= 1.0 {
            let (x, y) = to_pixels(ndc, width, height);
            return Projected::OnScreen { x, y };
        }
        return arrow(ndc, width, height);
    }
    // behind the camera the way to turn is the sideways offset itself
    arrow((lateral_x, lateral_y), width, height)
}

fn to_pixels(ndc: (f32, f32), width: f32, height: f32) -> (f32, f32) {
    (
        ((ndc.0 * 0.5 + 0.5) * width).round(),
        ((0.5 - ndc.1 * 0.5) * height).round(),
    )
}

fn arrow(direction: (f32, f32), width: f32, height: f32) -> Projected {
    let largest = direction.0.abs().max(direction.1.abs());
    let direction = if largest < 1.0e-6 {
        (0.0, -1.0)
    } else {
        direction
    };
    let largest = direction.0.abs().max(direction.1.abs());
    let scale = (1.0 - TAG_EDGE_MARGIN_FRACTION) / largest;
    let (x, y) = to_pixels((direction.0 * scale, direction.1 * scale), width, height);
    Projected::EdgeArrow {
        x,
        y,
        angle_radians: direction.1.atan2(direction.0),
    }
}

fn unit(vector: [f32; 2]) -> Option<[f32; 2]> {
    let length = vector[0].hypot(vector[1]);
    (length.is_finite() && length > 1.0e-6).then(|| [vector[0] / length, vector[1] / length])
}
