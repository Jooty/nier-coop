use crate::marker::{
    CameraState, EDGE_MARGIN_FRACTION, MAX_VERTICAL_FOV_RADIANS, MIN_VERTICAL_FOV_RADIANS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionMethod {
    ViewProjection,
    WorldMatrix,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projected {
    OnScreen { x: f32, y: f32 },
    EdgeArrow { x: f32, y: f32, angle_radians: f32 },
    Invalid,
}

struct Lateral {
    x: f32,
    y: f32,
    depth: f32,
}

pub fn project(
    camera: &CameraState,
    method: ProjectionMethod,
    point: [f32; 3],
    width: f32,
    height: f32,
) -> Projected {
    if !inputs_are_usable(camera, point, width, height) {
        return Projected::Invalid;
    }
    let lateral = match method {
        ProjectionMethod::ViewProjection => lateral_from_view_projection(camera, point),
        ProjectionMethod::WorldMatrix => lateral_from_world_matrix(camera, point),
    };
    if !(lateral.x.is_finite() && lateral.y.is_finite() && lateral.depth.is_finite()) {
        return Projected::Invalid;
    }
    if lateral.depth > camera.near {
        let ndc = (lateral.x / lateral.depth, lateral.y / lateral.depth);
        if ndc.0.abs() <= 1.0 && ndc.1.abs() <= 1.0 {
            let (x, y) = to_pixels(ndc, width, height);
            return Projected::OnScreen { x, y };
        }
        return arrow(ndc, width, height);
    }
    // behind the camera the screen direction to turn is the lateral offset itself
    arrow((lateral.x, lateral.y), width, height)
}

fn inputs_are_usable(camera: &CameraState, point: [f32; 3], width: f32, height: f32) -> bool {
    width > 0.0
        && height > 0.0
        && width.is_finite()
        && height.is_finite()
        && camera.aspect > 0.0
        && camera.aspect.is_finite()
        && camera.near.is_finite()
        && camera.vertical_fov.is_finite()
        && camera.vertical_fov > MIN_VERTICAL_FOV_RADIANS
        && camera.vertical_fov < MAX_VERTICAL_FOV_RADIANS
        && point.iter().all(|axis| axis.is_finite())
}

fn lateral_from_view_projection(camera: &CameraState, point: [f32; 3]) -> Lateral {
    let row = |index: usize| {
        let m = &camera.view_projection[index * 4..index * 4 + 4];
        m[0] * point[0] + m[1] * point[1] + m[2] * point[2] + m[3]
    };
    Lateral {
        x: row(0),
        y: row(1),
        depth: row(3),
    }
}

fn lateral_from_world_matrix(camera: &CameraState, point: [f32; 3]) -> Lateral {
    let m = &camera.world_matrix;
    let offset = [point[0] - m[3], point[1] - m[7], point[2] - m[11]];
    let dot = |a: usize, b: usize, c: usize| offset[0] * m[a] + offset[1] * m[b] + offset[2] * m[c];
    let tan_half = (camera.vertical_fov * 0.5).tan();
    Lateral {
        x: -dot(0, 4, 8) / (tan_half * camera.aspect),
        y: dot(1, 5, 9) / tan_half,
        depth: dot(2, 6, 10),
    }
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
    let scale = (1.0 - EDGE_MARGIN_FRACTION) / largest;
    let clamped = (direction.0 * scale, direction.1 * scale);
    let (x, y) = to_pixels(clamped, width, height);
    Projected::EdgeArrow {
        x,
        y,
        angle_radians: (-direction.1).atan2(direction.0),
    }
}
