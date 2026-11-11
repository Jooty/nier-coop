use replicant_proto::FightCameraAngle;

use crate::fight_sync::{
    CAMERA_ANGLE_MATCH_DEGREES, CAMERA_SETTLE_MS, FIGHT_CAMERA_ANGLE_KEEP_MS,
    GUEST_CAMERA_WAIT_AFTER_RECEIPT_MS, SNOW_CAMERA_INDEX,
};
use crate::memory::GameMemory;
use crate::side_view::CHANGE_CAMERA_NATIVE_RVA;
use crate::story_jump::{
    NATIVE_ARG_FIRST_OFFSET_BYTES, NATIVE_ARG_INTEGER_TYPE_TAG, NATIVE_ARG_REAL_TYPE_TAG,
    NATIVE_ARG_STRIDE_BYTES, NATIVE_ARG_VALUE_OFFSET_BYTES, NATIVE_ARGS_HEADER_BYTES,
};

pub const SET_FIXED_CAMERA_NATIVE_RVA: u32 = 0x6F_93E0;
pub const SET_FREE_CAMERA_DIRECTION_FIXED_NATIVE_RVA: u32 = 0x6F_AA20;
pub const SET_FREE_CAMERA_DISTANCE_OFFSET_NATIVE_RVA: u32 = 0x6F_AF20;
pub const RESET_FREE_CAMERA_DIRECTION_WITH_ACTOR_GROUP_NATIVE_RVA: u32 = 0x6F_2820;

const MIN_FORWARD_LENGTH: f32 = 1.0e-4;

/// The camera natives a fight script calls to set its angle, numbered as on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptedCameraNative {
    SetFixedCamera,
    ChangeCamera,
    SetFreeCameraDirectionFixed,
    SetFreeCameraDistanceOffset,
    ResetFreeCameraDirectionWithActorGroup,
}

/// What the guest does with the host's settled camera angle on this poll.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuestCameraStep {
    AlreadyMatches {
        angle: FightCameraAngle,
        gap_degrees: f32,
    },
    Apply {
        angle: FightCameraAngle,
        gap_degrees: f32,
        change_to_snow_camera: bool,
    },
    Verify {
        angle: FightCameraAngle,
        gap_degrees: f32,
    },
    Expired(FightCameraAngle),
}

/// The guest's own camera when it polls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuestCameraView {
    pub menu_open: bool,
    pub in_event_scene: bool,
    pub phase: Option<u16>,
    pub forward: Option<[f32; 3]>,
    pub active_camera: i32,
}

/// The host's side: after a camera native turns the view, the angle is read once the camera
/// settled, and later still while a pause menu stops the camera.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HostCameraSampler {
    due: Option<(ScriptedCameraNative, u64)>,
}

/// The guest's side: the host's latest settled angle, applied once the guest's own call had
/// time to land and no menu or scene is open.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GuestCameraFollow {
    pending: Option<(FightCameraAngle, u64)>,
    verify: Option<(FightCameraAngle, u64)>,
    last_own_call_ms: Option<u64>,
}

impl ScriptedCameraNative {
    pub const ALL: [Self; 5] = [
        Self::SetFixedCamera,
        Self::ChangeCamera,
        Self::SetFreeCameraDirectionFixed,
        Self::SetFreeCameraDistanceOffset,
        Self::ResetFreeCameraDirectionWithActorGroup,
    ];

    pub fn code(self) -> u8 {
        match self {
            Self::SetFixedCamera => 1,
            Self::ChangeCamera => 2,
            Self::SetFreeCameraDirectionFixed => 3,
            Self::SetFreeCameraDistanceOffset => 4,
            Self::ResetFreeCameraDirectionWithActorGroup => 5,
        }
    }

    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|native| native.code() == code)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::SetFixedCamera => "_SetFixedCamera",
            Self::ChangeCamera => "_ChangeCamera",
            Self::SetFreeCameraDirectionFixed => "_SetFreeCameraDirectionFixed",
            Self::SetFreeCameraDistanceOffset => "_SetFreeCameraDistanceOffset",
            Self::ResetFreeCameraDirectionWithActorGroup => {
                "_ResetFreeCameraDirectionWithActorGroup"
            }
        }
    }

    pub fn rva(self) -> u32 {
        match self {
            Self::SetFixedCamera => SET_FIXED_CAMERA_NATIVE_RVA,
            Self::ChangeCamera => CHANGE_CAMERA_NATIVE_RVA,
            Self::SetFreeCameraDirectionFixed => SET_FREE_CAMERA_DIRECTION_FIXED_NATIVE_RVA,
            Self::SetFreeCameraDistanceOffset => SET_FREE_CAMERA_DISTANCE_OFFSET_NATIVE_RVA,
            Self::ResetFreeCameraDirectionWithActorGroup => {
                RESET_FREE_CAMERA_DIRECTION_WITH_ACTOR_GROUP_NATIVE_RVA
            }
        }
    }

    pub fn argument_count(self) -> usize {
        match self {
            Self::SetFixedCamera => 6,
            Self::ChangeCamera => 3,
            Self::SetFreeCameraDirectionFixed => 11,
            Self::SetFreeCameraDistanceOffset => 5,
            Self::ResetFreeCameraDirectionWithActorGroup => 4,
        }
    }

    /// A distance offset leaves the direction alone; every other one may turn the view.
    pub fn turns_the_view(self) -> bool {
        self != Self::SetFreeCameraDistanceOffset
    }
}

impl HostCameraSampler {
    pub fn note_call(&mut self, native: ScriptedCameraNative, now_ms: u64) {
        if native.turns_the_view() {
            self.due = Some((native, now_ms + CAMERA_SETTLE_MS));
        }
    }

    /// The native whose settled angle should be read now. A pause menu stops the camera, so
    /// the read moves to one settle time after the menu closes.
    pub fn poll(&mut self, now_ms: u64, menu_open: bool) -> Option<ScriptedCameraNative> {
        let (native, due_ms) = self.due?;
        if menu_open {
            self.due = Some((native, due_ms.max(now_ms + CAMERA_SETTLE_MS)));
            return None;
        }
        if now_ms < due_ms {
            return None;
        }
        self.due = None;
        Some(native)
    }
}

impl GuestCameraFollow {
    pub fn receive(&mut self, angle: FightCameraAngle, now_ms: u64) {
        self.pending = Some((angle, now_ms));
        self.verify = None;
    }

    pub fn note_own_call(&mut self, now_ms: u64) {
        self.last_own_call_ms = Some(now_ms);
    }

    pub fn poll(&mut self, now_ms: u64, view: &GuestCameraView) -> Option<GuestCameraStep> {
        if let Some((angle, due_ms)) = self.verify
            && now_ms >= due_ms
            && !view.menu_open
        {
            self.verify = None;
            let gap_degrees = view.forward.map_or(f32::NAN, |forward| {
                angle_between_degrees(forward, angle.forward)
            });
            return Some(GuestCameraStep::Verify { angle, gap_degrees });
        }
        let (angle, received_ms) = self.pending?;
        if now_ms.saturating_sub(received_ms) > FIGHT_CAMERA_ANGLE_KEEP_MS
            || view.phase.is_some_and(|phase| phase != angle.phase)
        {
            self.pending = None;
            return Some(GuestCameraStep::Expired(angle));
        }
        let own_call_settled = self
            .last_own_call_ms
            .is_none_or(|at| now_ms.saturating_sub(at) >= CAMERA_SETTLE_MS);
        if view.menu_open
            || view.in_event_scene
            || !own_call_settled
            || now_ms.saturating_sub(received_ms) < GUEST_CAMERA_WAIT_AFTER_RECEIPT_MS
        {
            return None;
        }
        let forward = view.forward?;
        self.pending = None;
        let gap_degrees = angle_between_degrees(forward, angle.forward);
        let change_to_snow_camera =
            angle.active_camera == SNOW_CAMERA_INDEX && view.active_camera != SNOW_CAMERA_INDEX;
        if gap_degrees <= CAMERA_ANGLE_MATCH_DEGREES && !change_to_snow_camera {
            return Some(GuestCameraStep::AlreadyMatches { angle, gap_degrees });
        }
        self.verify = Some((angle, now_ms + CAMERA_SETTLE_MS));
        Some(GuestCameraStep::Apply {
            angle,
            gap_degrees,
            change_to_snow_camera,
        })
    }
}

/// The camera's look direction, column 2 of the published world matrix, as a unit vector.
pub fn forward_of_world_matrix(matrix: &[f32; 16]) -> Option<[f32; 3]> {
    normalized([matrix[2], matrix[6], matrix[10]])
}

/// Yaw about the up axis (0 looks along +Z, 90 along +X) and pitch (positive looks up), in degrees.
pub fn look_angles_degrees(forward: [f32; 3]) -> (f32, f32) {
    let yaw = forward[0].atan2(forward[2]).to_degrees();
    let pitch = forward[1].atan2(forward[0].hypot(forward[2])).to_degrees();
    (yaw, pitch)
}

pub fn angle_between_degrees(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (Some(a), Some(b)) = (normalized(a), normalized(b)) else {
        return f32::NAN;
    };
    let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    dot.clamp(-1.0, 1.0).acos().to_degrees()
}

pub fn direction_text(forward: Option<[f32; 3]>) -> String {
    forward.map_or_else(
        || "an unreadable direction".to_owned(),
        |forward| {
            let (yaw, pitch) = look_angles_degrees(forward);
            format!("yaw {yaw:.1} pitch {pitch:.1}")
        },
    )
}

/// The numbers a native was called with, read from its argument block (`None` for a slot that
/// is not a number). `arguments` is the native's own argument pointer.
pub fn read_native_numbers<M: GameMemory>(
    memory: &M,
    arguments: u64,
    count: usize,
) -> Vec<Option<f64>> {
    (0..count)
        .map(|index| {
            let slot = arguments
                + (NATIVE_ARGS_HEADER_BYTES
                    + NATIVE_ARG_FIRST_OFFSET_BYTES
                    + index * NATIVE_ARG_STRIDE_BYTES) as u64;
            let value = slot + NATIVE_ARG_VALUE_OFFSET_BYTES as u64;
            match memory.read_i32(slot).ok()?.cast_unsigned() {
                NATIVE_ARG_REAL_TYPE_TAG => memory.read_f64(value).ok(),
                NATIVE_ARG_INTEGER_TYPE_TAG => memory.read_i32(value).ok().map(f64::from),
                _ => None,
            }
        })
        .collect()
}

pub fn camera_call_line(
    native: ScriptedCameraNative,
    numbers: &[Option<f64>],
    forward_before: Option<[f32; 3]>,
    active_camera: Option<i32>,
    in_fight_window: bool,
) -> String {
    let arguments: Vec<String> = numbers
        .iter()
        .map(|number| number.map_or_else(|| "?".to_owned(), |number| format!("{number}")))
        .collect();
    format!(
        "scripted camera: {}({}) ran {}; the camera looked {} with camera {} active",
        native.name(),
        arguments.join(","),
        if in_fight_window {
            "in a fight window"
        } else {
            "outside a fight window"
        },
        direction_text(forward_before),
        active_camera.map_or_else(|| "?".to_owned(), |index| index.to_string())
    )
}

pub fn settled_camera_line(
    native: ScriptedCameraNative,
    forward: Option<[f32; 3]>,
    active_camera: Option<i32>,
    sent: bool,
) -> String {
    format!(
        "scripted camera: settled after {}, the camera looks {} with camera {} active{}",
        native.name(),
        direction_text(forward),
        active_camera.map_or_else(|| "?".to_owned(), |index| index.to_string()),
        if sent { ", sent to the guest" } else { "" }
    )
}

pub fn guest_camera_step_line(step: &GuestCameraStep, own: Option<[f32; 3]>) -> String {
    let host = |angle: &FightCameraAngle| {
        let native =
            ScriptedCameraNative::from_code(angle.native).map_or("?", |native| native.name());
        format!(
            "the host's camera after {native} looks {} with camera {} active",
            direction_text(Some(angle.forward)),
            angle.active_camera
        )
    };
    match step {
        GuestCameraStep::AlreadyMatches { angle, gap_degrees } => format!(
            "scripted camera: {}, yours looks {} ({gap_degrees:.1} degrees apart), already the same",
            host(angle),
            direction_text(own)
        ),
        GuestCameraStep::Apply {
            angle,
            gap_degrees,
            change_to_snow_camera,
        } => format!(
            "scripted camera: {}, yours looks {} ({gap_degrees:.1} degrees apart), the host's direction is set here{}",
            host(angle),
            direction_text(own),
            if *change_to_snow_camera {
                " and the free camera made active"
            } else {
                ""
            }
        ),
        GuestCameraStep::Verify { angle, gap_degrees } => format!(
            "scripted camera: after setting it, yours looks {} ({gap_degrees:.1} degrees from the host's {})",
            direction_text(own),
            direction_text(Some(angle.forward))
        ),
        GuestCameraStep::Expired(angle) => format!(
            "scripted camera: {} was dropped unapplied (too old or another phase)",
            host(angle)
        ),
    }
}

fn normalized(vector: [f32; 3]) -> Option<[f32; 3]> {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    (length.is_finite() && length > MIN_FORWARD_LENGTH)
        .then(|| [vector[0] / length, vector[1] / length, vector[2] / length])
}
