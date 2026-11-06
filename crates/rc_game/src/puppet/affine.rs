use crate::offsets::{
    ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_ROW_STRIDE, CREATION_MATRIX_BYTES,
};

/// A 3x4 row-major affine in the game's layout: each row is a basis lane triple plus the
/// translation in lane 3, as in the actor transform block at `+0x90`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
    pub rows: [[f32; 4]; 3],
}

impl Affine {
    pub const IDENTITY: Self = Self {
        rows: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ],
    };

    pub fn from_block(block: &[u8; ACTOR_TRANSFORM_BLOCK_BYTES]) -> Self {
        Self {
            rows: core::array::from_fn(|row| {
                core::array::from_fn(|lane| {
                    let at = row * ACTOR_TRANSFORM_ROW_STRIDE + lane * 4;
                    f32::from_le_bytes([block[at], block[at + 1], block[at + 2], block[at + 3]])
                })
            }),
        }
    }

    pub fn translation(offset: [f32; 3]) -> Self {
        let mut affine = Self::IDENTITY;
        for (row, value) in affine.rows.iter_mut().zip(offset) {
            row[3] = value;
        }
        affine
    }

    /// The game's Euler order: `Rz(z) * Ry(y) * Rx(x)`, angles in radians.
    pub fn rotation_xyz(angles: [f32; 3]) -> Self {
        let (sin_x, cos_x) = angles[0].sin_cos();
        let (sin_y, cos_y) = angles[1].sin_cos();
        let (sin_z, cos_z) = angles[2].sin_cos();
        Self {
            rows: [
                [
                    cos_z * cos_y,
                    cos_z * sin_y * sin_x - sin_z * cos_x,
                    cos_z * sin_y * cos_x + sin_z * sin_x,
                    0.0,
                ],
                [
                    sin_z * cos_y,
                    sin_z * sin_y * sin_x + cos_z * cos_x,
                    sin_z * sin_y * cos_x - cos_z * sin_x,
                    0.0,
                ],
                [-sin_y, cos_y * sin_x, cos_y * cos_x, 0.0],
            ],
        }
    }

    pub fn then(&self, local: &Self) -> Self {
        Self {
            rows: core::array::from_fn(|row| {
                core::array::from_fn(|lane| {
                    let turned: f32 = (0..3)
                        .map(|k| self.rows[row][k] * local.rows[k][lane])
                        .sum();
                    if lane == 3 {
                        turned + self.rows[row][3]
                    } else {
                        turned
                    }
                })
            }),
        }
    }

    pub fn with_rotation_of(&self, other: &Self) -> Self {
        Self {
            rows: core::array::from_fn(|row| {
                let mut lanes = other.rows[row];
                lanes[3] = self.rows[row][3];
                lanes
            }),
        }
    }

    pub fn position(&self) -> [f32; 3] {
        core::array::from_fn(|row| self.rows[row][3])
    }

    pub fn to_matrix_bytes(&self) -> [u8; CREATION_MATRIX_BYTES] {
        let mut bytes = [0_u8; CREATION_MATRIX_BYTES];
        let last_row = [0.0_f32, 0.0, 0.0, 1.0];
        for (lane, value) in self.rows.iter().flatten().chain(&last_row).enumerate() {
            bytes[lane * 4..lane * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}
