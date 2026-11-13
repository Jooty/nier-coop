use crate::story_follow::{
    REQUEST_WAITS_LOG_INTERVAL_MS, RequestReadiness, StoryRequest, request_waits_line,
};

#[derive(Debug, Clone, Default)]
pub(super) struct RequestWait {
    since_ms: Option<u64>,
    last_logged_ms: u64,
}

impl RequestWait {
    pub(super) fn note(
        &mut self,
        wanted: Option<StoryRequest>,
        can_take_a_request: bool,
        readiness: &RequestReadiness,
        now_ms: u64,
        logs: &mut Vec<String>,
    ) {
        let Some(request) = wanted.filter(|_| !can_take_a_request) else {
            self.since_ms = None;
            return;
        };
        let since_ms = *self.since_ms.get_or_insert_with(|| {
            self.last_logged_ms = now_ms;
            now_ms
        });
        if now_ms.saturating_sub(self.last_logged_ms) >= REQUEST_WAITS_LOG_INTERVAL_MS {
            self.last_logged_ms = now_ms;
            logs.push(request_waits_line(
                now_ms.saturating_sub(since_ms),
                &request,
                readiness,
            ));
        }
    }
}
