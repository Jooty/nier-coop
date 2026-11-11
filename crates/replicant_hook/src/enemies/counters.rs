use std::sync::atomic::{AtomicUsize, Ordering};

static HITS_SENT: AtomicUsize = AtomicUsize::new(0);
static LOCAL_ENEMY_HITS_BLOCKED: AtomicUsize = AtomicUsize::new(0);
static PLAYER_HITS_SENT: AtomicUsize = AtomicUsize::new(0);
static DROPS_SENT: AtomicUsize = AtomicUsize::new(0);
static COPY_DROPS_BLOCKED: AtomicUsize = AtomicUsize::new(0);

/// Running totals of the shared combat since the game started, for the overlay readout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CombatCounts {
    pub hits_sent: usize,
    pub hits_applied: usize,
    pub hits_refused: usize,
    pub local_enemy_hits_blocked: usize,
    pub player_hits_sent: usize,
    pub player_hits_applied: usize,
    pub kills_sent: usize,
    pub kills_applied: usize,
    pub drops_sent: usize,
    pub drops_applied: usize,
    pub copy_drops_blocked: usize,
    pub held: usize,
    pub aggro_on_guest: usize,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum HookCount {
    HitSent,
    LocalEnemyHitBlocked,
    PlayerHitSent,
    DropSent,
    CopyDropBlocked,
}

impl CombatCounts {
    pub const fn zero() -> Self {
        Self {
            hits_sent: 0,
            hits_applied: 0,
            hits_refused: 0,
            local_enemy_hits_blocked: 0,
            player_hits_sent: 0,
            player_hits_applied: 0,
            kills_sent: 0,
            kills_applied: 0,
            drops_sent: 0,
            drops_applied: 0,
            copy_drops_blocked: 0,
            held: 0,
            aggro_on_guest: 0,
        }
    }

    pub(super) fn with_hook_counts(mut self) -> Self {
        self.hits_sent = HITS_SENT.load(Ordering::Relaxed);
        self.local_enemy_hits_blocked = LOCAL_ENEMY_HITS_BLOCKED.load(Ordering::Relaxed);
        self.player_hits_sent = PLAYER_HITS_SENT.load(Ordering::Relaxed);
        self.drops_sent = DROPS_SENT.load(Ordering::Relaxed);
        self.copy_drops_blocked = COPY_DROPS_BLOCKED.load(Ordering::Relaxed);
        self
    }
}

pub(super) fn count(which: HookCount) {
    let counter = match which {
        HookCount::HitSent => &HITS_SENT,
        HookCount::LocalEnemyHitBlocked => &LOCAL_ENEMY_HITS_BLOCKED,
        HookCount::PlayerHitSent => &PLAYER_HITS_SENT,
        HookCount::DropSent => &DROPS_SENT,
        HookCount::CopyDropBlocked => &COPY_DROPS_BLOCKED,
    };
    counter.fetch_add(1, Ordering::Relaxed);
}
