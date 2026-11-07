use crate::marker::MATRIX_FLOATS;

pub const PROJECTION_DIAGONAL_TOLERANCE: f32 = 0.01;
const PROJECTION_X_SCALE_INDEX: usize = 0;
const PROJECTION_Y_SCALE_INDEX: usize = 5;

pub fn projection_is_cameras(
    projection: &[f32; MATRIX_FLOATS],
    vertical_fov: f32,
    aspect: f32,
) -> bool {
    if !(vertical_fov > 0.0 && aspect > 0.0) {
        return false;
    }
    let y_scale = 1.0 / (vertical_fov * 0.5).tan();
    let x_scale = y_scale / aspect;
    let close = |value: f32, wanted: f32| {
        (value - wanted).abs() <= wanted.abs() * PROJECTION_DIAGONAL_TOLERANCE
    };
    close(projection[PROJECTION_X_SCALE_INDEX], x_scale)
        && close(projection[PROJECTION_Y_SCALE_INDEX], y_scale)
}
