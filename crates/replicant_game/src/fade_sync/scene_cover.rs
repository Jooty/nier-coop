/// A layer this game covered by itself while the host played a scene this game was not in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneCover {
    pub layer: u64,
    pub covered_ms: u64,
    pub by: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverLift {
    FollowGaveUp,
    HostSceneEnded,
}

/// Covers left up for a scene this side never played; they are lifted when the follow gives up
/// or the host's scene ends, whichever comes first.
#[derive(Debug, Clone, Default)]
pub struct SceneCovers {
    covers: Vec<SceneCover>,
    host_was_in_scene: bool,
}

impl CoverLift {
    pub fn describe(self) -> &'static str {
        match self {
            Self::FollowGaveUp => "the scene follow gave up",
            Self::HostSceneEnded => "the host's scene ended",
        }
    }
}

impl SceneCovers {
    pub const fn new() -> Self {
        Self {
            covers: Vec::new(),
            host_was_in_scene: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.covers.is_empty()
    }

    /// Keeps a cover only while a follow is pending or the host is in a scene, and this game is not.
    pub fn note_cover(
        &mut self,
        layer: u64,
        by: &str,
        now_ms: u64,
        host_scene_running: bool,
        local_in_scene: bool,
    ) -> bool {
        if local_in_scene || !host_scene_running {
            return false;
        }
        self.covers.retain(|cover| cover.layer != layer);
        self.covers.push(SceneCover {
            layer,
            covered_ms: now_ms,
            by: by.to_owned(),
        });
        true
    }

    pub fn note_reveal(&mut self, layer: u64) {
        self.covers.retain(|cover| cover.layer != layer);
    }

    pub fn step(
        &mut self,
        host_in_scene: bool,
        follow_gave_up: bool,
        local_in_scene: bool,
    ) -> Option<(CoverLift, Vec<SceneCover>)> {
        let host_scene_ended = self.host_was_in_scene && !host_in_scene;
        self.host_was_in_scene = host_in_scene;
        if local_in_scene {
            self.covers.clear();
            return None;
        }
        let reason = if follow_gave_up {
            CoverLift::FollowGaveUp
        } else if host_scene_ended {
            CoverLift::HostSceneEnded
        } else {
            return None;
        };
        if self.covers.is_empty() {
            return None;
        }
        Some((reason, std::mem::take(&mut self.covers)))
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
