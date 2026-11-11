use crate::boss_messages::{BOSS_POST_PAIR_MEMORY_ENTRIES, BOSS_POST_PAIR_WINDOW_MS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NotedPost {
    message: u32,
    at_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PostPairs {
    own: Vec<NotedPost>,
    raised: Vec<NotedPost>,
}

impl PostPairs {
    pub const fn new() -> Self {
        Self {
            own: Vec::new(),
            raised: Vec::new(),
        }
    }

    pub fn note_own(&mut self, message: u32, now_ms: u64) -> Option<u64> {
        Self::note(&mut self.own, &mut self.raised, message, now_ms)
    }

    pub fn note_raised(&mut self, message: u32, now_ms: u64) -> Option<u64> {
        Self::note(&mut self.raised, &mut self.own, message, now_ms)
    }

    pub fn clear(&mut self) {
        self.own.clear();
        self.raised.clear();
    }

    fn note(
        mine: &mut Vec<NotedPost>,
        others: &mut Vec<NotedPost>,
        message: u32,
        now_ms: u64,
    ) -> Option<u64> {
        let fresh =
            |post: &NotedPost| now_ms.saturating_sub(post.at_ms) <= BOSS_POST_PAIR_WINDOW_MS;
        mine.retain(fresh);
        others.retain(fresh);
        let partner = others.iter().position(|post| post.message == message);
        if let Some(index) = partner {
            let partner = others.remove(index);
            return Some(now_ms.saturating_sub(partner.at_ms));
        }
        if mine.len() >= BOSS_POST_PAIR_MEMORY_ENTRIES {
            mine.remove(0);
        }
        mine.push(NotedPost {
            message,
            at_ms: now_ms,
        });
        None
    }
}
