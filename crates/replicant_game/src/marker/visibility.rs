use crate::marker::{
    CameraState, FADE_END_DISTANCE_UNITS, FADE_MINIMUM_ALPHA, FADE_START_DISTANCE_UNITS,
    MARKER_PEER_TIMEOUT_MS,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkerVisibility {
    pub enabled: bool,
    pub peer_known: bool,
    pub same_phase: bool,
    pub peer_state_age_ms: u64,
    pub camera: Option<CameraState>,
    pub event_scene: bool,
    pub world_frozen: bool,
}

pub fn hidden_reason(visibility: &MarkerVisibility) -> Option<&'static str> {
    if !visibility.enabled {
        return Some("turned off");
    }
    if !visibility.peer_known {
        return Some("no partner state");
    }
    if !visibility.same_phase {
        return Some("partner in another phase");
    }
    if visibility.peer_state_age_ms > MARKER_PEER_TIMEOUT_MS {
        return Some("partner state is stale");
    }
    if visibility.event_scene {
        return Some("cutscene running");
    }
    if visibility.world_frozen {
        return Some("game paused or in a menu");
    }
    match &visibility.camera {
        None => Some("camera unreadable"),
        Some(camera) if camera.active_index < 0 => Some("camera not ready"),
        Some(_) => None,
    }
}

pub fn fade_alpha(distance: f32) -> f32 {
    let span = FADE_END_DISTANCE_UNITS - FADE_START_DISTANCE_UNITS;
    let progress = ((distance - FADE_START_DISTANCE_UNITS) / span).clamp(0.0, 1.0);
    1.0 - progress * (1.0 - FADE_MINIMUM_ALPHA)
}
