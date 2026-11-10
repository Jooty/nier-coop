mod guest_hits;
mod guest_plan;
mod host_hits;
mod host_watch;
mod layout;
mod log_text;

pub use guest_hits::{GuestGaugeHits, measured_gauge_damage};
pub use guest_plan::{
    GuestGaugeFacts, GuestGaugeStep, GuestGaugeWait, HostGaugeView, LocalGaugeView,
    guest_gauge_step, guest_node_hp, guest_node_hp_after_own_hit, guest_remaining_floor,
};
pub use host_hits::{
    GaugeHitDrop, GaugeHitFacts, GaugeHitVerdict, HostGaugeHits, node_hp_after_damage,
};
pub use host_watch::HostGaugeWatch;
pub use layout::{
    GaugeNodeReading, GaugeReading, count_nodes, manager_address, read_gauge,
    read_hp_floor_percent, write_node_hp,
};
pub use log_text::{
    gauge_close_text, gauge_hit_applied_text, gauge_hit_dropped_text, gauge_open_text,
    guest_close_text, guest_open_readback_text, guest_open_text, guest_wait_text,
    own_gauge_hit_text,
};

pub const ATTACK_GAUGE_MANAGER_RVA: u32 = 0x11A_4240;
pub const ATTACK_GAUGE_HP_FLOOR_PERCENT_RVA: u32 = 0x11A_F328;
pub const ADD_ATTACK_GAUGE_NODE_RVA: u32 = 0x36_C660;
pub const OPEN_ATTACK_GAUGE_RVA: u32 = 0x36_F580;
pub const OPEN_TWO_ACTOR_ATTACK_GAUGE_RVA: u32 = 0x36_F0F0;
pub const CLOSE_ATTACK_GAUGE_RVA: u32 = 0x36_CF10;
pub const UPDATE_ATTACK_GAUGE_RVA: u32 = 0x36_D220;
pub const ATTACK_GAUGE_DUMMY_HIT_RVA: u32 = 0x36_EA80;
pub const BREAK_ATTACK_GAUGE_NODE_RVA: u32 = 0x36_CC70;
pub const ATTACK_GAUGE_MODE_OFFSET: u64 = 0x0;
pub const ATTACK_GAUGE_BOSS_HANDLE_OFFSET: u64 = 0x4;
pub const ATTACK_GAUGE_PARTNER_HANDLE_OFFSET: u64 = 0xC;
pub const ATTACK_GAUGE_TOTAL_SECONDS_OFFSET: u64 = 0x14;
pub const ATTACK_GAUGE_REMAINING_SECONDS_OFFSET: u64 = 0x18;
pub const ATTACK_GAUGE_DUMMY_HANDLE_OFFSET: u64 = 0x1C;
pub const ATTACK_GAUGE_FIRST_NODE_OFFSET: u64 = 0x40;
pub const ATTACK_GAUGE_MAP_POINTS_OFFSET: u64 = 0xB0B8;
pub const ATTACK_GAUGE_OPTIONS_OFFSET: u64 = 0xB0C4;
pub const ATTACK_GAUGE_NODE_BONE_OFFSET: u64 = 0x0;
pub const ATTACK_GAUGE_NODE_SIZE_OFFSET: u64 = 0x4;
pub const ATTACK_GAUGE_NODE_OFFSET_OFFSET: u64 = 0x10;
pub const ATTACK_GAUGE_NODE_KIND_OFFSET: u64 = 0x3C;
pub const ATTACK_GAUGE_NODE_EXTRA_OFFSET: u64 = 0x40;
pub const ATTACK_GAUGE_NODE_HP_OFFSET: u64 = 0x44;
pub const ATTACK_GAUGE_NODE_MAX_HP_OFFSET: u64 = 0x48;
pub const ATTACK_GAUGE_NODE_NEXT_OFFSET: u64 = 0x50;
pub const ATTACK_GAUGE_MODE_ONE_ACTOR: i32 = 1;
pub const ATTACK_GAUGE_MODE_TWO_ACTORS: i32 = 2;
pub const ATTACK_GAUGE_PERCENT_DIVISOR: f32 = 100.0;
pub const GUEST_GAUGE_REMAINING_FLOOR_SECONDS: f32 = 0.5;
pub const GUEST_ONLY_GAUGE_GRACE_MS: u64 = 1500;
pub const HOST_GAUGE_RESEND_INTERVAL_MS: u64 = 250;
pub const GUEST_GAUGE_HIT_REPEAT_WINDOW_MS: u64 = 2000;
pub const GAUGE_HIT_SEQ_MEMORY_ENTRIES: usize = 128;
pub const GUEST_GAUGE_MEASURING_HP: i32 = 1_000_000;
