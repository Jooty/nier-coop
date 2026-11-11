use serde::{Deserialize, Serialize};

use crate::partner::{
    HIGHEST_POD_AIM_EYE_COORDINATE, LENGTH_OF_THE_AIM_RAY_GIVEN_TO_A_POD_SHOT,
    POD_AIM_DIRECTION_LENGTH_ERROR_ALLOWED, SCREEN_DEPTH_OF_THE_FAR_AIM_POINT,
    SCREEN_DEPTH_OF_THE_NEAR_AIM_POINT, SHORTEST_AIM_RAY_THAT_GIVES_A_DIRECTION,
};

/// The ray a player's pod aims along: the world point the camera ray starts at, and its unit direction.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PodAim {
    pub eye: [f32; 3],
    pub direction: [f32; 3],
}

/// The two world points a pod shot asks the camera for, kept as they come in.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AimRayPoints {
    near: Option<[f32; 3]>,
    far: Option<[f32; 3]>,
}

impl PodAim {
    /// None when the two points give no direction: not finite, or closer than a ray can be.
    pub fn from_ray_points(near: [f32; 3], far: [f32; 3]) -> Option<Self> {
        let along = [far[0] - near[0], far[1] - near[1], far[2] - near[2]];
        let length = length_of(along);
        if !length.is_finite() || length < SHORTEST_AIM_RAY_THAT_GIVES_A_DIRECTION {
            return None;
        }
        Self {
            eye: near,
            direction: along.map(|axis| axis / length),
        }
        .checked()
    }

    /// None for an aim no camera gives: a value that is not finite, an eye outside the world,
    /// or a direction that is not one unit long.
    pub fn checked(self) -> Option<Self> {
        let eye_is_in_the_world = self
            .eye
            .iter()
            .all(|axis| axis.is_finite() && axis.abs() <= HIGHEST_POD_AIM_EYE_COORDINATE);
        let length = length_of(self.direction);
        let direction_is_a_unit =
            length.is_finite() && (length - 1.0).abs() <= POD_AIM_DIRECTION_LENGTH_ERROR_ALLOWED;
        (eye_is_in_the_world && direction_is_a_unit).then_some(self)
    }

    /// The world point a pod shot gets for a screen depth in place of the local camera's own:
    /// the eye at the near depth, a point along the aim at the far depth.
    pub fn ray_point(self, screen_depth: f32) -> Option<[f32; 3]> {
        let aim = self.checked()?;
        let depths = SCREEN_DEPTH_OF_THE_NEAR_AIM_POINT..=SCREEN_DEPTH_OF_THE_FAR_AIM_POINT;
        if !depths.contains(&screen_depth) {
            return None;
        }
        let along = screen_depth * LENGTH_OF_THE_AIM_RAY_GIVEN_TO_A_POD_SHOT;
        Some(core::array::from_fn(|axis| {
            aim.eye[axis] + aim.direction[axis] * along
        }))
    }
}

impl AimRayPoints {
    pub const fn new() -> Self {
        Self {
            near: None,
            far: None,
        }
    }

    /// Keeps the point when the depth is one of the two a pod shot asks for.
    pub fn note(&mut self, screen_depth: f32, world_point: [f32; 3]) {
        if screen_depth == SCREEN_DEPTH_OF_THE_NEAR_AIM_POINT {
            self.near = Some(world_point);
        } else if screen_depth == SCREEN_DEPTH_OF_THE_FAR_AIM_POINT {
            self.far = Some(world_point);
        }
    }

    pub fn aim(&self) -> Option<PodAim> {
        PodAim::from_ray_points(self.near?, self.far?)
    }
}

fn length_of(vector: [f32; 3]) -> f32 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}
