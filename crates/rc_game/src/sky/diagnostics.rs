use crate::memory::{GameMemory, MemoryError};
use crate::offsets::{
    BLEND_SKIP_ONCE_RVA, DIAGNOSTIC_COUNTERS_COUNT, DIAGNOSTIC_COUNTERS_FIRST,
    ENVIRONMENT_APPLIED_RECORD_OFFSET, ENVIRONMENT_ELAPSED_SECONDS_OFFSET, ENVIRONMENT_MANAGER_RVA,
    ENVIRONMENT_RECORD_BYTES, ENVIRONMENT_RECORD_LOGGED_FLOATS_OFFSET,
    ENVIRONMENT_TABLE_POINTER_OFFSET, ENVIRONMENT_TABLE_RECORDS, ENVIRONMENT_TO_RECORD_OFFSET,
    EVENT_ENVIRONMENT_FLAG_RVA, EVENT_ENVIRONMENT_NAME_RVA, GAME_COUNTERS_RVA, LOAD_MAP_NAME_RVA,
    MAP_NAME_READ_LEN, POINTER_MASK_HIGHEST, POINTER_MASK_LOWEST,
    RENDER_PARAMS_FLOAT_BLOCK_OFFSETS, RENDER_PARAMS_FLOATS_PER_BLOCK, RENDER_PARAMS_RVA,
};
use crate::sky::{SkyReading, read_resource_state, read_sky};

const FNV_OFFSET_BASIS: u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;
const RECORD_FLOATS_LOGGED: usize = 8;
const RENDER_FLOATS_LOGGED: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct SkyDiagnostics {
    pub reading: SkyReading,
    pub elapsed_seconds: f32,
    pub resource_state: Option<i32>,
    pub applied_record_hash: u64,
    pub to_record_hash: u64,
    pub table_record_hash: Option<u64>,
    pub applied_masked_hash: u64,
    pub to_masked_hash: u64,
    pub table_masked_hash: Option<u64>,
    pub applied_record_floats: [f32; RECORD_FLOATS_LOGGED],
    pub render_floats: [f32; RENDER_FLOATS_LOGGED],
    pub blend_skip_once: u8,
    pub counters: [u8; DIAGNOSTIC_COUNTERS_COUNT],
    pub map_name: String,
    pub event_environment: Option<String>,
    pub table_hashes: Vec<u32>,
}

impl SkyDiagnostics {
    pub fn table_line(&self) -> String {
        let hashes: Vec<String> = self
            .table_hashes
            .iter()
            .enumerate()
            .map(|(area, hash)| format!("{area}:{hash:08x}"))
            .collect();
        format!(
            "map {:?} event_env {:?} table {}",
            self.map_name,
            self.event_environment,
            hashes.join(" ")
        )
    }

    pub fn line(&self) -> String {
        let reading = &self.reading;
        let table = self
            .table_record_hash
            .map_or_else(|| "none".to_owned(), |hash| format!("{hash:016x}"));
        let counters: Vec<String> = self.counters.iter().map(u8::to_string).collect();
        let table_masked = self
            .table_masked_hash
            .map_or_else(|| "none".to_owned(), |hash| format!("{hash:016x}"));
        let floats = format_floats(&self.applied_record_floats);
        let render = format_floats(&self.render_floats);
        format!(
            "env {} blend {:.2} elapsed {:.2} reload {} wind {} weather {}/{} resources {:?} ready {} \
             applied {:016x} to {:016x} table {table} masked applied {:016x} to {:016x} table {table_masked} skip {} \
             record_floats [{}] render_floats [{}] counters {:#x}.. [{}]",
            reading.env_area,
            reading.env_blend_seconds,
            self.elapsed_seconds,
            reading.table_reloading,
            reading.wind_area,
            reading.weather,
            reading.weather_offset,
            self.resource_state,
            reading.sky_resources_ready,
            self.applied_record_hash,
            self.to_record_hash,
            self.applied_masked_hash,
            self.to_masked_hash,
            self.blend_skip_once,
            floats,
            render,
            DIAGNOSTIC_COUNTERS_FIRST,
            counters.join(","),
        )
    }
}

fn format_floats(values: &[f32]) -> String {
    let texts: Vec<String> = values.iter().map(|value| format!("{value:.3}")).collect();
    texts.join(" ")
}

pub fn masked_fnv1a_64(bytes: &[u8]) -> u64 {
    let mut masked = bytes.to_vec();
    let (words, _) = masked.as_chunks_mut::<8>();
    for word in words {
        let value = u64::from_le_bytes(*word);
        if (POINTER_MASK_LOWEST..=POINTER_MASK_HIGHEST).contains(&value) {
            *word = [0; 8];
        }
    }
    fnv1a_64(&masked)
}

fn read_text<M: GameMemory>(memory: &M, address: u64) -> String {
    let bytes = memory
        .read_array::<MAP_NAME_READ_LEN>(address)
        .unwrap_or([0; MAP_NAME_READ_LEN]);
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn read_table_hashes<M: GameMemory>(memory: &M, table: u64) -> Vec<u32> {
    if table == 0 {
        return Vec::new();
    }
    (0..ENVIRONMENT_TABLE_RECORDS)
        .map(|area| {
            let record = table + area * ENVIRONMENT_RECORD_BYTES as u64;
            memory
                .read_array::<ENVIRONMENT_RECORD_BYTES>(record)
                .map_or(0, |bytes| masked_fnv1a_64(&bytes) as u32)
        })
        .collect()
}

pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

pub fn read_sky_diagnostics<M: GameMemory>(memory: &M) -> Result<SkyDiagnostics, MemoryError> {
    let reading = read_sky(memory)?;
    let base = memory.module_base();
    let environment = base + u64::from(ENVIRONMENT_MANAGER_RVA);
    let applied = memory
        .read_array::<ENVIRONMENT_RECORD_BYTES>(environment + ENVIRONMENT_APPLIED_RECORD_OFFSET)?;
    let to = memory
        .read_array::<ENVIRONMENT_RECORD_BYTES>(environment + ENVIRONMENT_TO_RECORD_OFFSET)?;
    let table = memory.read_u64(environment + ENVIRONMENT_TABLE_POINTER_OFFSET)?;
    let table_record = u64::try_from(reading.env_area)
        .ok()
        .filter(|_| table != 0)
        .and_then(|area| {
            let record = table + area * ENVIRONMENT_RECORD_BYTES as u64;
            memory.read_array::<ENVIRONMENT_RECORD_BYTES>(record).ok()
        });
    let mut applied_record_floats = [0.0_f32; RECORD_FLOATS_LOGGED];
    let logged = &applied[ENVIRONMENT_RECORD_LOGGED_FLOATS_OFFSET..];
    let (chunks, _) = logged.as_chunks::<4>();
    for (slot, chunk) in applied_record_floats.iter_mut().zip(chunks) {
        *slot = f32::from_le_bytes(*chunk);
    }
    let mut render_floats = [0.0_f32; RENDER_FLOATS_LOGGED];
    let render = base + u64::from(RENDER_PARAMS_RVA);
    for (block, offset) in RENDER_PARAMS_FLOAT_BLOCK_OFFSETS.iter().enumerate() {
        for index in 0..RENDER_PARAMS_FLOATS_PER_BLOCK {
            let at = render + offset + index as u64 * 4;
            render_floats[block * RENDER_PARAMS_FLOATS_PER_BLOCK + index] =
                memory.read_f32(at).unwrap_or(0.0);
        }
    }
    let counters = memory.read_array::<DIAGNOSTIC_COUNTERS_COUNT>(
        base + u64::from(GAME_COUNTERS_RVA) + u64::from(DIAGNOSTIC_COUNTERS_FIRST),
    )?;
    Ok(SkyDiagnostics {
        elapsed_seconds: memory.read_f32(environment + ENVIRONMENT_ELAPSED_SECONDS_OFFSET)?,
        resource_state: read_resource_state(memory),
        applied_record_hash: fnv1a_64(&applied),
        to_record_hash: fnv1a_64(&to),
        table_record_hash: table_record.as_ref().map(|bytes| fnv1a_64(bytes)),
        applied_masked_hash: masked_fnv1a_64(&applied),
        to_masked_hash: masked_fnv1a_64(&to),
        table_masked_hash: table_record.as_ref().map(|bytes| masked_fnv1a_64(bytes)),
        applied_record_floats,
        render_floats,
        blend_skip_once: memory.read_u8(base + u64::from(BLEND_SKIP_ONCE_RVA))?,
        counters,
        map_name: read_text(memory, base + u64::from(LOAD_MAP_NAME_RVA)),
        event_environment: (memory
            .read_u8(base + u64::from(EVENT_ENVIRONMENT_FLAG_RVA))
            .unwrap_or(0)
            != 0)
            .then(|| read_text(memory, base + u64::from(EVENT_ENVIRONMENT_NAME_RVA))),
        table_hashes: read_table_hashes(memory, table),
        reading,
    })
}
