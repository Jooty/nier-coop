use crate::marker::{
    CAMERA_ACTIVE_INDEX_RVA, CAMERA_ASPECT_RVA, CAMERA_FAR_RVA, CAMERA_FOV_RVA, CAMERA_MATRIX_RVA,
    CAMERA_NEAR_RVA, MATRIX_FLOATS, VIEW_PROJECTION_RVA,
};
use crate::memory::{GameMemory, MemoryError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraState {
    pub world_matrix: [f32; MATRIX_FLOATS],
    pub view_projection: [f32; MATRIX_FLOATS],
    pub vertical_fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub active_index: i32,
}

pub fn read_camera_state<M: GameMemory>(memory: &M) -> Result<Option<CameraState>, MemoryError> {
    let at = |rva: u32| memory.module_base() + u64::from(rva);
    let world_matrix = read_stable_matrix(memory, at(CAMERA_MATRIX_RVA))?;
    let Some(world_matrix) = world_matrix else {
        return Ok(None);
    };
    Ok(Some(CameraState {
        world_matrix,
        view_projection: read_matrix(memory, at(VIEW_PROJECTION_RVA))?,
        vertical_fov: memory.read_f32(at(CAMERA_FOV_RVA))?,
        aspect: memory.read_f32(at(CAMERA_ASPECT_RVA))?,
        near: memory.read_f32(at(CAMERA_NEAR_RVA))?,
        far: memory.read_f32(at(CAMERA_FAR_RVA))?,
        active_index: memory.read_i32(at(CAMERA_ACTIVE_INDEX_RVA))?,
    }))
}

fn read_stable_matrix<M: GameMemory>(
    memory: &M,
    address: u64,
) -> Result<Option<[f32; MATRIX_FLOATS]>, MemoryError> {
    // the game thread may be mid-write; two equal reads mean no tear
    let first = read_matrix(memory, address)?;
    let second = read_matrix(memory, address)?;
    Ok((first == second).then_some(first))
}

fn read_matrix<M: GameMemory>(
    memory: &M,
    address: u64,
) -> Result<[f32; MATRIX_FLOATS], MemoryError> {
    let bytes = memory.read_array::<{ MATRIX_FLOATS * 4 }>(address)?;
    Ok(core::array::from_fn(|index| {
        let at = index * 4;
        f32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    }))
}
