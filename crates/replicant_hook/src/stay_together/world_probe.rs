use replicant_game::offsets::{
    COLLISION_HIT_BUFFER_BYTES, COLLISION_HIT_FLAG_OFFSET, COLLISION_HIT_NORMAL_OFFSET,
    COLLISION_HIT_POINT_OFFSET, COLLISION_QUERY_SEGMENT_FUNCTION_RVA,
    COLLISION_QUERY_SPHERE_FUNCTION_RVA, COLLISION_TERRAIN_LAYER_MASK,
};
use replicant_game::stay_together::{
    BODY_FLOOR_CLEARANCE, BODY_RADIUS, FLOOR_RAY_END_BELOW_PARTNER, FLOOR_RAY_START_ABOVE_PARTNER,
    FloorHit, WorldProbe,
};

use crate::build_gate::VerifiedBuild;

const FLOAT_BYTES: usize = 4;
const Y_LANE: usize = FLOAT_BYTES;

type QuerySegment = unsafe extern "system" fn(*mut u8, *const f32, *const f32, u32) -> u8;
type QuerySphere = unsafe extern "system" fn(*mut u8, *const f32, f32, u32) -> u8;

#[repr(C, align(16))]
struct AlignedVector([f32; 4]);

#[repr(C, align(16))]
struct HitBuffer([u8; COLLISION_HIT_BUFFER_BYTES]);

pub(crate) struct CollisionProbe {
    query_segment: usize,
    query_sphere: usize,
}

impl CollisionProbe {
    pub(crate) fn new(build: &VerifiedBuild) -> Self {
        let base = build.module_base();
        Self {
            query_segment: (base + u64::from(COLLISION_QUERY_SEGMENT_FUNCTION_RVA)) as usize,
            query_sphere: (base + u64::from(COLLISION_QUERY_SPHERE_FUNCTION_RVA)) as usize,
        }
    }
}

impl WorldProbe for CollisionProbe {
    fn floor_below(&self, x: f32, y: f32, z: f32) -> Option<FloorHit> {
        let from = AlignedVector([x, y + FLOOR_RAY_START_ABOVE_PARTNER, z, 0.0]);
        let to = AlignedVector([x, y - FLOOR_RAY_END_BELOW_PARTNER, z, 0.0]);
        let mut hit = HitBuffer([0; COLLISION_HIT_BUFFER_BYTES]);
        #[expect(
            unsafe_code,
            reason = "calls the game's CollisionManager::QuerySegment at its verified address"
        )]
        // SAFETY: the build gate passed, so the RVA starts QuerySegment (hit buffer, two aligned
        // four-float vectors, layer mask). The hit buffer is zeroed and larger than the 0x58-byte
        // record, and this runs on the game thread.
        unsafe {
            let query: QuerySegment =
                core::mem::transmute::<usize, QuerySegment>(self.query_segment);
            query(
                hit.0.as_mut_ptr(),
                from.0.as_ptr(),
                to.0.as_ptr(),
                COLLISION_TERRAIN_LAYER_MASK,
            );
        }
        (hit.0[COLLISION_HIT_FLAG_OFFSET] == 1).then(|| FloorHit {
            y: read_f32(&hit.0, COLLISION_HIT_POINT_OFFSET + Y_LANE),
            normal_y: read_f32(&hit.0, COLLISION_HIT_NORMAL_OFFSET + Y_LANE),
        })
    }

    fn body_fits(&self, x: f32, y: f32, z: f32) -> bool {
        let center = AlignedVector([x, y + BODY_RADIUS + BODY_FLOOR_CLEARANCE, z, 0.0]);
        let mut hit = HitBuffer([0; COLLISION_HIT_BUFFER_BYTES]);
        #[expect(
            unsafe_code,
            reason = "calls the game's CollisionManager::QuerySphere at its verified address"
        )]
        // SAFETY: the build gate passed, so the RVA starts QuerySphere (hit buffer, aligned
        // four-float centre, radius, layer mask). The hit buffer is zeroed and large enough, and
        // this runs on the game thread.
        let overlapped = unsafe {
            let query: QuerySphere = core::mem::transmute::<usize, QuerySphere>(self.query_sphere);
            query(
                hit.0.as_mut_ptr(),
                center.0.as_ptr(),
                BODY_RADIUS,
                COLLISION_TERRAIN_LAYER_MASK,
            )
        };
        overlapped == 0
    }
}

fn read_f32(buffer: &[u8; COLLISION_HIT_BUFFER_BYTES], offset: usize) -> f32 {
    let mut lane = [0_u8; FLOAT_BYTES];
    lane.copy_from_slice(&buffer[offset..offset + FLOAT_BYTES]);
    f32::from_le_bytes(lane)
}
