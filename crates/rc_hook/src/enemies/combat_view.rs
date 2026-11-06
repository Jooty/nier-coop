use std::cell::Cell;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};

use rc_proto::EnemyId;

use crate::enemies::EnemySyncRole;

static VIEW: LazyLock<Mutex<CombatView>> = LazyLock::new(Mutex::default);

thread_local! {
    static APPLYING_REMOTE: Cell<bool> = const { Cell::new(false) };
}

/// What the combat hooks need to know about this frame: which local actors are the shared
/// enemies (the host's real ones, or the guest's copies) and who the two player bodies are.
#[derive(Debug, Clone, Default)]
pub(super) struct CombatView {
    pub(super) role: Option<EnemySyncRole>,
    pub(super) phase: u16,
    pub(super) enemies: HashMap<u64, EnemyId>,
    pub(super) local_player: u64,
    pub(super) puppet: u64,
}

struct RemoteGuard(bool);

impl CombatView {
    pub(super) fn is_enemy(&self, actor: Option<u64>) -> Option<EnemyId> {
        actor.and_then(|actor| self.enemies.get(&actor).copied())
    }
}

impl Drop for RemoteGuard {
    fn drop(&mut self) {
        APPLYING_REMOTE.with(|flag| flag.set(self.0));
    }
}

pub(super) fn publish_view(view: CombatView) {
    *VIEW.lock().unwrap_or_else(PoisonError::into_inner) = view;
}

pub(super) fn clear_view() {
    publish_view(CombatView::default());
}

pub(super) fn with_view<T>(read: impl FnOnce(&CombatView) -> T) -> T {
    read(&VIEW.lock().unwrap_or_else(PoisonError::into_inner))
}

pub(super) fn applying_remote() -> bool {
    APPLYING_REMOTE.with(Cell::get)
}

pub(super) fn as_remote<T>(apply: impl FnOnce() -> T) -> T {
    let _restore = RemoteGuard(APPLYING_REMOTE.with(|flag| flag.replace(true)));
    apply()
}
