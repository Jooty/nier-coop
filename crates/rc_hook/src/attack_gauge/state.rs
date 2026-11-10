use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr};
use std::sync::{LazyLock, Mutex, OnceLock};
use std::time::Instant;

use rc_game::attack_gauge::{GuestGaugeHits, GuestGaugeWait, HostGaugeHits, HostGaugeWatch};
use rc_proto::{AttackGaugeOpen, AttackGaugeState};

use crate::build_gate::VerifiedBuild;

pub(super) static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
pub(super) static ORIGINAL_DUMMY_HIT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
pub(super) static ORIGINAL_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
pub(super) static GUEST_MIRRORING: AtomicBool = AtomicBool::new(false);
pub(super) static GUEST: Mutex<GuestSide> = Mutex::new(GuestSide::new());
pub(super) static HOST: Mutex<HostSide> = Mutex::new(HostSide::new());
static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

pub(super) struct GuestSide {
    pub(super) phase: u16,
    pub(super) host: Option<AttackGaugeState>,
    pub(super) host_fresh: bool,
    pub(super) mirrored_serial: Option<u32>,
    pub(super) own_gauge_since_ms: Option<u64>,
    pub(super) last_wait: Option<GuestGaugeWait>,
    pub(super) gave_up_serial: Option<u32>,
    pub(super) count_mismatch_logged: bool,
    pub(super) hits: GuestGaugeHits,
}

pub(super) struct HostSide {
    pub(super) watch: HostGaugeWatch,
    pub(super) hits: HostGaugeHits,
    pub(super) last_open: Option<AttackGaugeOpen>,
    pub(super) unshared_boss_logged: Option<u64>,
    pub(super) long_list_logged: bool,
}

impl GuestSide {
    const fn new() -> Self {
        Self {
            phase: 0,
            host: None,
            host_fresh: false,
            mirrored_serial: None,
            own_gauge_since_ms: None,
            last_wait: None,
            gave_up_serial: None,
            count_mismatch_logged: false,
            hits: GuestGaugeHits::new(),
        }
    }
}

impl HostSide {
    const fn new() -> Self {
        Self {
            watch: HostGaugeWatch::new(),
            hits: HostGaugeHits::new(),
            last_open: None,
            unshared_boss_logged: None,
            long_list_logged: false,
        }
    }
}

pub(super) fn now_ms() -> u64 {
    u64::try_from(EPOCH.elapsed().as_millis()).unwrap_or(u64::MAX)
}
