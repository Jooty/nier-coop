use crate::offsets::{
    ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_ROW_STRIDE, CREATION_MATRIX_BYTES,
};

const SMALLEST_TRUSTED_SCALE: f32 = 0.001;

/// The transform to give a guest enemy so it stands at the host's `position` facing `yaw`,
/// keeping the uniform scale its own transform block (`actor + 0x90`) already has.
pub fn shadow_matrix(
    current_block: &[u8; ACTOR_TRANSFORM_BLOCK_BYTES],
    position: [f32; 3],
    yaw_radians: f32,
) -> [u8; CREATION_MATRIX_BYTES] {
    scaled_matrix(position, yaw_radians, horizontal_scale(current_block))
}

pub fn scaled_matrix(
    position: [f32; 3],
    yaw_radians: f32,
    scale: f32,
) -> [u8; CREATION_MATRIX_BYTES] {
    let scale = if scale.is_finite() && scale >= SMALLEST_TRUSTED_SCALE {
        scale
    } else {
        1.0
    };
    let (sin, cos) = yaw_radians.sin_cos();
    let [x, y, z] = position;
    let rows: [[f32; 4]; 4] = [
        [cos * scale, 0.0, -sin * scale, x],
        [0.0, scale, 0.0, y],
        [sin * scale, 0.0, cos * scale, z],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut bytes = [0_u8; CREATION_MATRIX_BYTES];
    for (lane, value) in rows.iter().flatten().enumerate() {
        bytes[lane * 4..lane * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn horizontal_scale(block: &[u8; ACTOR_TRANSFORM_BLOCK_BYTES]) -> f32 {
    let lane =
        |at: usize| f32::from_le_bytes([block[at], block[at + 1], block[at + 2], block[at + 3]]);
    let row2 = 2 * ACTOR_TRANSFORM_ROW_STRIDE;
    let scale = lane(row2).hypot(lane(row2 + 8));
    if scale.is_finite() && scale >= SMALLEST_TRUSTED_SCALE {
        scale
    } else {
        1.0
    }
}

pub fn creation_matrix_scale(matrix: &[u8; CREATION_MATRIX_BYTES]) -> f32 {
    let mut block = [0_u8; ACTOR_TRANSFORM_BLOCK_BYTES];
    block.copy_from_slice(&matrix[..ACTOR_TRANSFORM_BLOCK_BYTES]);
    horizontal_scale(&block)
}
