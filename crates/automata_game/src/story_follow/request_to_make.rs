use crate::load_gate::StoryPlace;
use crate::story_follow::StoryRequest;

pub(super) fn request_to_make(request: &StoryRequest, current: &StoryPlace) -> StoryRequest {
    match request {
        // A sub-phase change to another phase id would skip the scene teardown.
        StoryRequest::SubPhase { id, name } if *id != current.phase_id => {
            StoryRequest::full_with_default_params(*id, name)
        }
        other => other.clone(),
    }
}
