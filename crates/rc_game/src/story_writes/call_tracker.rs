use crate::story_calls::name_hash;
use crate::story_writes::{
    MAX_TALK_QUEUED_NAMES, TALK_CALLBACK_ATTRIBUTION_MAX_MS, TALK_QUEUED_NAME_LIFETIME_MS,
    TALK_TAG_FUNCTION_NAME_MAX_BYTES,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunningCall {
    pub name_hash: u64,
    pub from_talk_tag: bool,
    pub started_ms: u64,
}

/// Which Lua function the game started last, and whether a talk line's tag queued it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptCallTracker {
    talk_queued: Vec<(u64, u64)>,
    running: Option<RunningCall>,
}

impl ScriptCallTracker {
    pub const fn new() -> Self {
        Self {
            talk_queued: Vec::new(),
            running: None,
        }
    }

    pub fn note_talk_tag_call(&mut self, function_name: &[u8], now_ms: u64) {
        let Some(hash) = hash_of(function_name) else {
            return;
        };
        self.talk_queued.retain(|&(_, queued_ms)| {
            now_ms.saturating_sub(queued_ms) < TALK_QUEUED_NAME_LIFETIME_MS
        });
        if self.talk_queued.len() >= MAX_TALK_QUEUED_NAMES {
            self.talk_queued.remove(0);
        }
        self.talk_queued.push((hash, now_ms));
    }

    pub fn start_call(&mut self, function_name: &[u8], now_ms: u64) -> Option<RunningCall> {
        let hash = hash_of(function_name)?;
        let queued_at = self.talk_queued.iter().position(|&(queued, queued_ms)| {
            queued == hash && now_ms.saturating_sub(queued_ms) < TALK_QUEUED_NAME_LIFETIME_MS
        });
        let from_talk_tag = queued_at.is_some();
        if let Some(position) = queued_at {
            self.talk_queued.remove(position);
        }
        let call = RunningCall {
            name_hash: hash,
            from_talk_tag,
            started_ms: now_ms,
        };
        self.running = Some(call);
        Some(call)
    }

    pub fn queued_by_talk_tag(&self, function_name: &[u8], now_ms: u64) -> bool {
        let Some(hash) = hash_of(function_name) else {
            return false;
        };
        self.talk_queued.iter().any(|&(queued, queued_ms)| {
            queued == hash && now_ms.saturating_sub(queued_ms) < TALK_QUEUED_NAME_LIFETIME_MS
        })
    }

    pub fn running(&self) -> Option<RunningCall> {
        self.running
    }

    pub fn talk_callback_running(&self, now_ms: u64) -> bool {
        self.running.is_some_and(|call| {
            call.from_talk_tag
                && now_ms.saturating_sub(call.started_ms) < TALK_CALLBACK_ATTRIBUTION_MAX_MS
        })
    }
}

/// The Lua function a `<CB=Function,arg,...>` tag body names.
pub fn talk_tag_function_name(body: &[u8]) -> &[u8] {
    let end = body
        .iter()
        .position(|&byte| byte == b',' || byte == 0)
        .unwrap_or(body.len())
        .min(TALK_TAG_FUNCTION_NAME_MAX_BYTES);
    body[..end].trim_ascii()
}

fn hash_of(function_name: &[u8]) -> Option<u64> {
    let name = core::str::from_utf8(function_name).ok()?;
    (!name.is_empty()).then(|| name_hash(name))
}
