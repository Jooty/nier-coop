use std::collections::BTreeMap;

pub const BOSS_HIT_LINES_PER_WINDOW: u32 = 4;
pub const BOSS_HIT_LINE_WINDOW_MS: u64 = 1000;
pub const BOSS_HIT_SUMMARY_WINDOW_MS: u64 = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BossHitPlayer {
    Host,
    Guest,
}

#[derive(Debug, Default)]
pub struct BossHitLog {
    line_window_start_ms: u64,
    lines_in_window: u32,
    summary_window_start_ms: Option<u64>,
    totals: BTreeMap<(BossHitPlayer, i32), (u32, i64)>,
}

impl BossHitLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_line(&mut self, now_ms: u64) -> bool {
        if now_ms.saturating_sub(self.line_window_start_ms) >= BOSS_HIT_LINE_WINDOW_MS {
            self.line_window_start_ms = now_ms;
            self.lines_in_window = 0;
        }
        if self.lines_in_window >= BOSS_HIT_LINES_PER_WINDOW {
            return false;
        }
        self.lines_in_window += 1;
        true
    }

    pub fn record(
        &mut self,
        now_ms: u64,
        player: BossHitPlayer,
        kind: i32,
        damage: i32,
    ) -> Option<String> {
        let summary = self.summary_if_window_ended(now_ms);
        self.summary_window_start_ms.get_or_insert(now_ms);
        let entry = self.totals.entry((player, kind)).or_default();
        entry.0 += 1;
        entry.1 += i64::from(damage);
        summary
    }

    fn summary_if_window_ended(&mut self, now_ms: u64) -> Option<String> {
        let start = self.summary_window_start_ms?;
        if now_ms.saturating_sub(start) < BOSS_HIT_SUMMARY_WINDOW_MS {
            return None;
        }
        self.summary_window_start_ms = None;
        let parts: Vec<String> = std::mem::take(&mut self.totals)
            .into_iter()
            .map(|((player, kind), (count, total))| {
                let who = match player {
                    BossHitPlayer::Host => "host",
                    BossHitPlayer::Guest => "guest",
                };
                format!("{who} kind {kind}: {count} hits, damage {total}")
            })
            .collect();
        (!parts.is_empty()).then(|| format!("boss hits, last 2 s: {}", parts.join("; ")))
    }
}
