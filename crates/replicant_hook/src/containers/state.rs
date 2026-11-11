use std::sync::{LazyLock, Mutex, OnceLock};
use std::time::Instant;

use replicant_game::containers::{ContainerRegistry, FlagHold, OpenWindow, OpenedSet};

pub(super) static REGISTRY: LazyLock<Mutex<ContainerRegistry>> = LazyLock::new(Mutex::default);
pub(super) static WINDOW: LazyLock<Mutex<OpenWindow>> = LazyLock::new(Mutex::default);
pub(super) static OPENED: LazyLock<Mutex<OpenedSet>> = LazyLock::new(Mutex::default);
pub(super) static HELD: LazyLock<Mutex<FlagHold>> = LazyLock::new(Mutex::default);

static STARTED_AT: OnceLock<Instant> = OnceLock::new();

pub(super) fn now_ms() -> u64 {
    u64::try_from(STARTED_AT.get_or_init(Instant::now).elapsed().as_millis()).unwrap_or(u64::MAX)
}
