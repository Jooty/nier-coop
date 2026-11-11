use crate::enemies::attack_record::ActorHandle;
use crate::memory::GameMemory;

pub const CA_GAUGE_ENTRIES_RVA: u32 = 0x12D_B150;
pub const CA_GAUGE_MANAGER_RVA: u32 = 0x12D_B140;
pub const CA_GAUGE_MANAGER_RAW_BYTES: usize = 32;
pub const CA_GAUGE_HEARTBEAT_INTERVAL_MS: u64 = 5000;
pub const CA_GAUGE_ENTRY_COUNT: usize = 64;
pub const CA_GAUGE_ENTRY_SIZE: usize = 0x60;
pub const CA_GAUGE_SCAN_INTERVAL_MS: u64 = 250;
pub const CA_GAUGE_CHANGE_LINES_PER_SECOND: u32 = 4;
const CA_GAUGE_CHANGE_WINDOW_MS: u64 = 1000;
const CA_GAUGE_HANDLE_OFFSET: usize = 4;
const CA_GAUGE_VALUE_OFFSET: usize = 0x4C;
const CA_GAUGE_TIMER_OFFSET: usize = 0x50;

pub type CaGaugeRaw = [u8; CA_GAUGE_ENTRY_SIZE];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaGaugeEntry {
    pub in_use: bool,
    pub handle: ActorHandle,
    pub value: f32,
    pub timer: f32,
    pub raw: CaGaugeRaw,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaGaugeEvent {
    Opened { index: usize, handle: ActorHandle },
    Closed { index: usize },
    Changed { index: usize, entry: CaGaugeEntry },
}

#[derive(Debug, Clone, Default)]
struct WatchedSlot {
    last_seen: Option<CaGaugeRaw>,
    window_start_ms: u64,
    lines_in_window: u32,
}

#[derive(Debug, Clone)]
pub struct CaGaugeWatch {
    slots: Vec<WatchedSlot>,
}

impl Default for CaGaugeWatch {
    fn default() -> Self {
        Self::new()
    }
}

impl CaGaugeEntry {
    pub fn parse(raw: &CaGaugeRaw) -> Self {
        let float = |offset: usize| {
            f32::from_le_bytes([
                raw[offset],
                raw[offset + 1],
                raw[offset + 2],
                raw[offset + 3],
            ])
        };
        let word = |offset: usize| {
            u32::from_le_bytes([
                raw[offset],
                raw[offset + 1],
                raw[offset + 2],
                raw[offset + 3],
            ])
        };
        Self {
            in_use: raw[0] != 0,
            handle: ActorHandle {
                index: word(CA_GAUGE_HANDLE_OFFSET),
                generation: word(CA_GAUGE_HANDLE_OFFSET + 4),
            },
            value: float(CA_GAUGE_VALUE_OFFSET),
            timer: float(CA_GAUGE_TIMER_OFFSET),
            raw: *raw,
        }
    }

    pub fn read<M: GameMemory>(memory: &M, index: usize) -> Option<Self> {
        let address = memory.module_base()
            + u64::from(CA_GAUGE_ENTRIES_RVA)
            + (index * CA_GAUGE_ENTRY_SIZE) as u64;
        let raw = memory.read_array::<CA_GAUGE_ENTRY_SIZE>(address).ok()?;
        Some(Self::parse(&raw))
    }

    pub fn raw_hex(&self) -> String {
        self.raw.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

impl CaGaugeWatch {
    pub fn new() -> Self {
        Self {
            slots: vec![WatchedSlot::default(); CA_GAUGE_ENTRY_COUNT],
        }
    }

    pub fn observe(
        &mut self,
        now_ms: u64,
        index: usize,
        entry: &CaGaugeEntry,
    ) -> Option<CaGaugeEvent> {
        let slot = self.slots.get_mut(index)?;
        match (slot.last_seen, entry.in_use) {
            (None, true) => {
                slot.last_seen = Some(entry.raw);
                slot.window_start_ms = now_ms;
                slot.lines_in_window = 0;
                Some(CaGaugeEvent::Opened {
                    index,
                    handle: entry.handle,
                })
            }
            (Some(_), false) => {
                slot.last_seen = None;
                Some(CaGaugeEvent::Closed { index })
            }
            (Some(previous), true) if previous != entry.raw => {
                if now_ms.saturating_sub(slot.window_start_ms) >= CA_GAUGE_CHANGE_WINDOW_MS {
                    slot.window_start_ms = now_ms;
                    slot.lines_in_window = 0;
                }
                if slot.lines_in_window >= CA_GAUGE_CHANGE_LINES_PER_SECOND {
                    return None;
                }
                slot.lines_in_window += 1;
                slot.last_seen = Some(entry.raw);
                Some(CaGaugeEvent::Changed {
                    index,
                    entry: *entry,
                })
            }
            _ => None,
        }
    }
}

pub fn ca_entry_for_actor<M: GameMemory>(memory: &M, actor: u64) -> Option<(usize, CaGaugeEntry)> {
    (0..CA_GAUGE_ENTRY_COUNT).find_map(|index| {
        let entry = CaGaugeEntry::read(memory, index)?;
        (entry.in_use && entry.handle.resolve(memory) == Some(actor)).then_some((index, entry))
    })
}

pub fn ca_gauge_hit_text<M: GameMemory>(memory: &M, actor: u64) -> String {
    ca_entry_for_actor(memory, actor).map_or_else(
        || "ca none".to_owned(),
        |(index, entry)| format!("ca {index} +0x4C {} +0x50 {}", entry.value, entry.timer),
    )
}

pub fn ca_gauge_probe_on_text(module_base: u64) -> String {
    format!(
        "ca gauge: probe on, entries at {:#x}, manager {:#x}",
        module_base + u64::from(CA_GAUGE_ENTRIES_RVA),
        module_base + u64::from(CA_GAUGE_MANAGER_RVA)
    )
}

pub fn read_ca_gauge_manager_raw<M: GameMemory>(
    memory: &M,
) -> Option<[u8; CA_GAUGE_MANAGER_RAW_BYTES]> {
    memory
        .read_array::<CA_GAUGE_MANAGER_RAW_BYTES>(
            memory.module_base() + u64::from(CA_GAUGE_MANAGER_RVA),
        )
        .ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaGaugeScanStats {
    pub in_use: usize,
    pub first_read_ok: bool,
    pub read_failures: u64,
}

pub fn ca_gauge_heartbeat_text(
    stats: &CaGaugeScanStats,
    manager_raw: Option<&[u8; CA_GAUGE_MANAGER_RAW_BYTES]>,
) -> String {
    let manager = manager_raw.map_or_else(
        || "unreadable".to_owned(),
        |raw| raw.iter().map(|byte| format!("{byte:02x}")).collect(),
    );
    format!(
        "ca gauge: heartbeat, in use {} of {CA_GAUGE_ENTRY_COUNT}, first read {}, read failures {}, manager raw {manager}",
        stats.in_use,
        if stats.first_read_ok { "ok" } else { "failed" },
        stats.read_failures
    )
}
