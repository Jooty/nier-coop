use crate::scene_follow::{
    GUEST_TAKES_HOST_VIEW_WITHIN_UNITS, HOST_VIEW_RELEASED_AFTER_GUEST_FAR_MS,
};
use crate::story_calls::Whereabouts;

/// What the guest's game knows when one of the host's scripted camera or fade calls runs here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostViewFacts {
    pub is_guest: bool,
    pub called_by_the_mod: bool,
    pub in_fight_window: bool,
    pub in_event_scene: bool,
    /// The handler holding the script slot was led here by the host's game.
    pub host_led_run: bool,
    /// A switch back to the free camera, or a fade in: it only gives the guest its view back.
    pub gives_the_view_back: bool,
    pub guest_to_host_units: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HostViewVerdict {
    /// Not a host's call this rule covers: it runs as it always did.
    NotGated,
    Apply {
        units: f32,
    },
    /// The guest is not near the host (`None`: in another phase or not known); its own view stays.
    Leave {
        units: Option<f32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostViewKind {
    Camera,
    Fade,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostViewRelease {
    pub kind: HostViewKind,
    pub run: String,
}

/// What the host's calls left on the guest's view: a fixed camera, or a screen faded out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostViewHold {
    camera_run: Option<String>,
    fade_run: Option<String>,
    far_since_ms: Option<u64>,
}

impl HostViewVerdict {
    pub fn judge(facts: &HostViewFacts) -> Self {
        let gated = facts.is_guest
            && !facts.called_by_the_mod
            && !facts.in_fight_window
            && !facts.in_event_scene
            && facts.host_led_run
            && !facts.gives_the_view_back;
        if !gated {
            return Self::NotGated;
        }
        match facts.guest_to_host_units {
            Some(units) if units.is_finite() && units <= GUEST_TAKES_HOST_VIEW_WITHIN_UNITS => {
                Self::Apply { units }
            }
            units => Self::Leave { units },
        }
    }

    pub fn runs(self) -> bool {
        !matches!(self, Self::Leave { .. })
    }
}

impl HostViewHold {
    pub const fn new() -> Self {
        Self {
            camera_run: None,
            fade_run: None,
            far_since_ms: None,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn holds_any(&self) -> bool {
        self.camera_run.is_some() || self.fade_run.is_some()
    }

    /// A host's camera call ran here; `leaves_free_camera` is a switch back to the free camera.
    pub fn host_camera_applied(&mut self, run: &str, leaves_free_camera: bool) {
        self.camera_run = (!leaves_free_camera).then(|| run.to_owned());
    }

    pub fn host_fade_applied(&mut self, run: &str, fading_out: bool) {
        self.fade_run = fading_out.then(|| run.to_owned());
    }

    /// The guest's own script set the camera: it is the guest's view again.
    pub fn own_camera_call(&mut self) {
        self.camera_run = None;
    }

    pub fn own_fade_call(&mut self) {
        self.fade_run = None;
    }

    /// Once per tick with how far the guest is from the host; what is due to be released.
    pub fn step(&mut self, guest_to_host_units: Option<f32>, now_ms: u64) -> Vec<HostViewRelease> {
        let near = guest_to_host_units
            .is_some_and(|units| units.is_finite() && units <= GUEST_TAKES_HOST_VIEW_WITHIN_UNITS);
        if near || !self.holds_any() {
            self.far_since_ms = None;
            return Vec::new();
        }
        let far_since = *self.far_since_ms.get_or_insert(now_ms);
        if now_ms.saturating_sub(far_since) < HOST_VIEW_RELEASED_AFTER_GUEST_FAR_MS {
            return Vec::new();
        }
        self.far_since_ms = None;
        let camera = self.camera_run.take().map(|run| HostViewRelease {
            kind: HostViewKind::Camera,
            run,
        });
        let fade = self.fade_run.take().map(|run| HostViewRelease {
            kind: HostViewKind::Fade,
            run,
        });
        camera.into_iter().chain(fade).collect()
    }
}

/// Straight-line distance between the guest and the host, or `None` when either is not known or
/// they are in different phases.
pub fn guest_to_host_units(guest: Option<Whereabouts>, host: Option<Whereabouts>) -> Option<f32> {
    let (guest, host) = (guest?, host?);
    if guest.phase != host.phase {
        return None;
    }
    let units = guest
        .position
        .iter()
        .zip(host.position)
        .map(|(here, there)| (here - there) * (here - there))
        .sum::<f32>()
        .sqrt();
    units.is_finite().then_some(units)
}
