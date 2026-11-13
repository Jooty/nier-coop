use crate::fight_sync::FIXED_CAMERA_INDEX;
use crate::fight_sync::camera_follow::{FixedCameraPose, HOST_CAMERA_APPLIED_AFTER_TICKS};

pub const CAMERA_AIM_MOVE_SENT_AGAIN_WORLD_UNITS: f32 = 0.05;
pub const CAMERA_AIM_SENT_AGAIN_AFTER_MS: u64 = 100;

/// A pose-only update of the host's fixed camera while it stays active: no switch comes with
/// it. A higher `seq` replaces a lower one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostCameraAim {
    pub seq: u32,
    pub pose: FixedCameraPose,
}

/// What the guest does with a pose update it received.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraAimReceipt {
    Kept,
    OlderThanTheLast,
    OutsideAFightWindow,
}

/// The host's side: sends the fixed camera's pose again when it moved since the last one sent
/// (with a switch or alone), at most once per `CAMERA_AIM_SENT_AGAIN_AFTER_MS`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HostCameraAimResend {
    next_seq: u32,
    last_sent: Option<(FixedCameraPose, u64)>,
}

/// The guest's side: keeps the newest pose update and applies it one tick later, only while
/// its own active camera is the fixed one. It never switches cameras.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GuestCameraAimFollow {
    pending: Option<(HostCameraAim, u32)>,
    last_seq: Option<u32>,
}

impl HostCameraAimResend {
    pub const fn new() -> Self {
        Self {
            next_seq: 0,
            last_sent: None,
        }
    }

    /// A switch message carried `pose` at `now_ms`: later updates are measured from it.
    pub fn sent_with_a_switch(&mut self, pose: FixedCameraPose, now_ms: u64) {
        self.last_sent = Some((pose, now_ms));
    }

    /// The pose update to send this tick, if the fixed camera is active and its pose moved.
    pub fn update_for(
        &mut self,
        in_window: bool,
        active_camera: Option<i32>,
        pose: Option<FixedCameraPose>,
        now_ms: u64,
    ) -> Option<HostCameraAim> {
        if !in_window {
            self.window_closed();
            return None;
        }
        let pose = pose?;
        if active_camera != Some(FIXED_CAMERA_INDEX) {
            return None;
        }
        if let Some((last, at_ms)) = self.last_sent {
            if !pose_moved(last, pose) {
                return None;
            }
            if now_ms.saturating_sub(at_ms) < CAMERA_AIM_SENT_AGAIN_AFTER_MS {
                return None;
            }
        }
        self.last_sent = Some((pose, now_ms));
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        Some(HostCameraAim { seq, pose })
    }

    pub fn window_closed(&mut self) {
        self.last_sent = None;
    }
}

impl GuestCameraAimFollow {
    pub const fn new() -> Self {
        Self {
            pending: None,
            last_seq: None,
        }
    }

    pub fn receive(&mut self, aim: HostCameraAim, in_window: bool) -> CameraAimReceipt {
        if !in_window {
            return CameraAimReceipt::OutsideAFightWindow;
        }
        if self.last_seq.is_some_and(|last| aim.seq <= last) {
            return CameraAimReceipt::OlderThanTheLast;
        }
        self.last_seq = Some(aim.seq);
        self.pending = Some((aim, 0));
        CameraAimReceipt::Kept
    }

    /// Called once a tick: `Ok` is a pose to set on the fixed camera now (no switch), `Err` a
    /// kept update dropped because the guest's own active camera is not the fixed one.
    pub fn tick(
        &mut self,
        in_window: bool,
        own_camera: Option<i32>,
    ) -> Option<Result<FixedCameraPose, HostCameraAim>> {
        if !in_window {
            self.window_closed();
            return None;
        }
        let (aim, ticks) = self.pending.as_mut()?;
        if *ticks < HOST_CAMERA_APPLIED_AFTER_TICKS {
            *ticks += 1;
            return None;
        }
        let aim = *aim;
        self.pending = None;
        Some(if own_camera == Some(FIXED_CAMERA_INDEX) {
            Ok(aim.pose)
        } else {
            Err(aim)
        })
    }

    pub fn window_closed(&mut self) {
        self.pending = None;
        self.last_seq = None;
    }
}

pub fn pose_moved(from: FixedCameraPose, to: FixedCameraPose) -> bool {
    from.eye
        .iter()
        .chain(&from.look_at)
        .zip(to.eye.iter().chain(&to.look_at))
        .any(|(a, b)| (a - b).abs() > CAMERA_AIM_MOVE_SENT_AGAIN_WORLD_UNITS)
}
