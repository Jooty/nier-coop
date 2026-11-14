use crate::load_gate::StoryPlace;
use crate::story_follow::{
    AskVerdict, RequestReadiness, StoryAsk, StoryAskAnswer, StoryMove, StoryRequest,
};

pub fn move_told_line(story_move: &StoryMove) -> String {
    format!(
        "story: told the partner move {} from {}: {}",
        story_move.number,
        story_move.from.describe(),
        story_move.request.describe()
    )
}

pub fn move_heard_line(story_move: &StoryMove, queued: usize) -> String {
    format!(
        "story: heard the host's move {} from {}: {} ({queued} queued)",
        story_move.number,
        story_move.from.describe(),
        story_move.request.describe()
    )
}

pub fn request_kept_line(number: u32, request: &StoryRequest) -> String {
    format!(
        "story: this game asked for {}, swallowed and kept as ask {number}",
        request.describe()
    )
}

pub fn request_replaced_line(old_number: u32, new_number: u32) -> String {
    format!("story: ask {old_number} is replaced by the newer ask {new_number}")
}

pub fn ask_sent_line(ask: &StoryAsk) -> String {
    format!(
        "story: asked the host (ask {}) at {} for {}",
        ask.number,
        ask.at.describe(),
        ask.request.describe()
    )
}

pub fn ask_kept_waiting_line(number: u32, waited_ms: u64, request: &StoryRequest) -> String {
    format!(
        "story: ask {number} kept for {waited_ms} ms, still no word from the host: {}",
        request.describe()
    )
}

pub fn answer_heard_line(answer: &StoryAskAnswer, request: &StoryRequest) -> String {
    format!(
        "story: the host answered ask {}: {} ({})",
        answer.number,
        answer.verdict.label(),
        request.describe()
    )
}

pub fn gave_up_line(number: u32, waited_ms: u64, request: &StoryRequest) -> String {
    format!(
        "story: no move and no answer for ask {number} in {waited_ms} ms, the request runs here: {}",
        request.describe()
    )
}

pub fn made_here_line(why: &str, request: &StoryRequest, delay_ms: u64) -> String {
    format!(
        "story: made here after {delay_ms} ms ({why}): {}",
        request.describe()
    )
}

pub fn move_dropped_as_already_here_line(number: u32, request: &StoryRequest) -> String {
    format!(
        "story: the host's move {number} is already here, dropped: {}",
        request.describe()
    )
}

pub fn catch_up_line(
    host_place: &StoryPlace,
    this_place: &StoryPlace,
    differing_ms: u64,
    request: &StoryRequest,
) -> String {
    format!(
        "story: the host has been at {} for {differing_ms} ms while this game is at {}, catching up: {}",
        host_place.describe(),
        this_place.describe(),
        request.describe()
    )
}

pub fn link_lost_line(had_a_kept_request: bool, queued: usize) -> String {
    format!(
        "story: the partner is gone, {queued} moves cleared, {}",
        if had_a_kept_request {
            "the kept request runs here"
        } else {
            "no request was kept"
        }
    )
}

pub fn in_flight_expired_line(target: &StoryPlace) -> String {
    format!(
        "story: the request for {} did not arrive in time, no longer counted as under way",
        target.describe()
    )
}

pub fn host_ask_made_line(number: u32, request: &StoryRequest) -> String {
    format!(
        "story: made for the guest's ask {number} in the host's game: {}",
        request.describe()
    )
}

pub fn host_ask_answered_line(number: u32, verdict: AskVerdict) -> String {
    format!("story: answered ask {number}: {}", verdict.label())
}

pub fn request_not_told_line(request: &StoryRequest) -> String {
    format!(
        "story: a request the other game could not take is passed unseen: {}",
        request.describe()
    )
}

pub fn request_waits_line(
    waited_ms: u64,
    request: &StoryRequest,
    readiness: &RequestReadiness,
) -> String {
    format!(
        "story: waiting {waited_ms} ms to make {}: this game cannot take a request (scene state {}, phase state {}, requested id {}, pending {}, current {})",
        request.describe(),
        readiness.scene_state,
        readiness.phase_system_state,
        readiness.requested_phase_id,
        phase_with_name(
            readiness.pending_phase_id,
            readiness.pending_name.as_deref()
        ),
        phase_with_name(
            readiness.current_phase_id,
            readiness.current_name.as_deref()
        ),
    )
}

pub fn pending_unequal_to_current_tolerated_line(
    differed_ms: u64,
    readiness: &RequestReadiness,
) -> String {
    format!(
        "story: pending {} has differed from current {} for {differed_ms} ms with the phase system idle, requests are made anyway",
        phase_with_name(
            readiness.pending_phase_id,
            readiness.pending_name.as_deref()
        ),
        phase_with_name(
            readiness.current_phase_id,
            readiness.current_name.as_deref()
        ),
    )
}

fn phase_with_name(id: i32, name: Option<&str>) -> String {
    format!("{id:#X} {}", name.unwrap_or("(no name)"))
}

pub fn watching_request_kept_line(number: u32, request: &StoryRequest) -> String {
    format!(
        "story: this copy watches, so its own request for {} is kept as ask {number} and not asked of the host; the host's move releases it",
        request.describe()
    )
}

pub fn watching_request_still_kept_line(
    number: u32,
    waited_ms: u64,
    request: &StoryRequest,
) -> String {
    format!(
        "story: this copy watches, ask {number} is still kept after {waited_ms} ms, not asked and not run here: {}",
        request.describe()
    )
}

pub fn watching_request_dropped_line(number: u32, request: &StoryRequest, why: &str) -> String {
    format!(
        "story: ask {number} kept while this copy watched is dropped ({why}): {}",
        request.describe()
    )
}
