use crate::memory::GameMemory;
use crate::move_log::{
    SPEC_TABLE_BYTE_COUNT, SPEC_TABLE_POINTER_OFFSET, SPEC_TABLE_WORDS_PER_LINE,
};
use crate::stay_together::FollowRole;

pub const SPEC_DUMP_SECOND_DUMP_DELAY_MS: u64 = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecDumpMoment {
    PhaseLoad,
    FiveSecondsLater,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecTable {
    pub pointer: u64,
    pub words: Option<Vec<f32>>,
}

#[derive(Debug, Default)]
pub struct SpecDumpSchedule {
    actor: u64,
    phase_loads: u32,
    second_dump_due_ms: Option<u64>,
}

impl SpecDumpMoment {
    pub fn label(self) -> &'static str {
        match self {
            Self::PhaseLoad => "phase load",
            Self::FiveSecondsLater => "5 s later",
        }
    }
}

impl SpecDumpSchedule {
    pub fn observe(&mut self, actor: u64, phase_loads: u32, now_ms: u64) -> Option<SpecDumpMoment> {
        if actor == 0 {
            self.actor = 0;
            self.phase_loads = 0;
            self.second_dump_due_ms = None;
            return None;
        }
        if actor != self.actor || phase_loads != self.phase_loads {
            self.actor = actor;
            self.phase_loads = phase_loads;
            self.second_dump_due_ms = Some(now_ms + SPEC_DUMP_SECOND_DUMP_DELAY_MS);
            return Some(SpecDumpMoment::PhaseLoad);
        }
        if self.second_dump_due_ms.is_some_and(|due| now_ms >= due) {
            self.second_dump_due_ms = None;
            return Some(SpecDumpMoment::FiveSecondsLater);
        }
        None
    }
}

pub fn read_spec_table<M: GameMemory>(memory: &M, actor: u64) -> Option<SpecTable> {
    let pointer = memory.read_u64(actor + SPEC_TABLE_POINTER_OFFSET).ok()?;
    if pointer == 0 {
        return Some(SpecTable {
            pointer,
            words: None,
        });
    }
    let mut bytes = vec![0_u8; SPEC_TABLE_BYTE_COUNT];
    let words = memory.read(pointer, &mut bytes).ok().map(|()| {
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|word| f32::from_le_bytes(*word))
            .collect()
    });
    Some(SpecTable { pointer, words })
}

pub fn role_label(role: FollowRole) -> &'static str {
    match role {
        FollowRole::Host => "host",
        FollowRole::Guest => "guest",
        FollowRole::Unknown => "unknown",
    }
}

pub fn format_spec_dump(
    role: &str,
    action_set: &str,
    seed: &str,
    moment: SpecDumpMoment,
    table: Option<&SpecTable>,
) -> Vec<String> {
    let head = format!(
        "move spec: role {role}, action_set {action_set}, seed {seed}, when {}",
        moment.label()
    );
    let Some(table) = table else {
        return vec![format!("{head}, table pointer unreadable")];
    };
    if table.pointer == 0 {
        return vec![format!("{head}, table null (no dump)")];
    }
    let Some(words) = &table.words else {
        return vec![format!(
            "{head}, table {:#x}, unreadable (no dump)",
            table.pointer
        )];
    };
    let mut lines = vec![format!("{head}, table {:#x}", table.pointer)];
    for (row, chunk) in words.chunks(SPEC_TABLE_WORDS_PER_LINE).enumerate() {
        let offset = row * SPEC_TABLE_WORDS_PER_LINE * 4;
        let values: Vec<String> = chunk.iter().map(|value| format!("{value:.4}")).collect();
        lines.push(format!("move spec +{offset:#05x}: {}", values.join(" ")));
    }
    lines
}
