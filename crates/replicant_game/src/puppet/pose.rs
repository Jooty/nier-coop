use crate::game::Position;
use crate::offsets::{
    ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_ROW_STRIDE, CREATION_MATRIX_BYTES,
};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PuppetPose {
    pub position: Position,
    pub yaw: f32,
}

impl PuppetPose {
    pub fn from_transform_block(block: &[u8; ACTOR_TRANSFORM_BLOCK_BYTES]) -> Self {
        let row = |index: usize| -> [f32; 4] {
            let start = index * ACTOR_TRANSFORM_ROW_STRIDE;
            core::array::from_fn(|lane| {
                let at = start + lane * 4;
                f32::from_le_bytes([block[at], block[at + 1], block[at + 2], block[at + 3]])
            })
        };
        let (row0, row1, row2) = (row(0), row(1), row(2));
        Self {
            position: Position {
                x: row0[3],
                y: row1[3],
                z: row2[3],
            },
            yaw: row2[0].atan2(row2[2]),
        }
    }

    pub fn creation_matrix(self) -> [u8; CREATION_MATRIX_BYTES] {
        let (sin, cos) = self.yaw.sin_cos();
        let rows: [[f32; 4]; 4] = [
            [cos, 0.0, -sin, self.position.x],
            [0.0, 1.0, 0.0, self.position.y],
            [sin, 0.0, cos, self.position.z],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let mut bytes = [0_u8; CREATION_MATRIX_BYTES];
        for (lane, value) in rows.iter().flatten().enumerate() {
            bytes[lane * 4..lane * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}
