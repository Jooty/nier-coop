use crate::memory::{GameMemory, MemoryError};
use crate::offsets::{
    ENVIRONMENT_AREA_OFFSET, ENVIRONMENT_BLEND_SECONDS_OFFSET, ENVIRONMENT_MANAGER_RVA,
    ENVIRONMENT_TABLE_RELOAD_OFFSET, WEATHER_COUNTER_RVA, WEATHER_OFFSET_COUNTER_RVA,
    WIND_AREA_OFFSET, WIND_MANAGER_RVA,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyReading {
    pub env_area: i32,
    pub env_blend_seconds: f32,
    pub wind_area: i32,
    pub weather: u8,
    pub weather_offset: u8,
    pub table_reloading: bool,
}

impl SkyReading {
    pub fn is_settled(&self) -> bool {
        self.env_area >= 0 && !self.table_reloading
    }
}

pub fn read_sky<M: GameMemory>(memory: &M) -> Result<SkyReading, MemoryError> {
    let base = memory.module_base();
    let environment = base + u64::from(ENVIRONMENT_MANAGER_RVA);
    let wind = base + u64::from(WIND_MANAGER_RVA);
    Ok(SkyReading {
        env_area: memory.read_i32(environment + ENVIRONMENT_AREA_OFFSET)?,
        env_blend_seconds: memory.read_f32(environment + ENVIRONMENT_BLEND_SECONDS_OFFSET)?,
        wind_area: memory.read_i32(wind + WIND_AREA_OFFSET)?,
        weather: memory.read_u8(base + u64::from(WEATHER_COUNTER_RVA))?,
        weather_offset: memory.read_u8(base + u64::from(WEATHER_OFFSET_COUNTER_RVA))?,
        table_reloading: memory.read_i32(environment + ENVIRONMENT_TABLE_RELOAD_OFFSET)? == 1,
    })
}

pub fn write_weather_counters<M: GameMemory>(
    memory: &mut M,
    weather: u8,
    weather_offset: u8,
) -> Result<(), MemoryError> {
    let base = memory.module_base();
    memory.write_u8(base + u64::from(WEATHER_COUNTER_RVA), weather)?;
    memory.write_u8(base + u64::from(WEATHER_OFFSET_COUNTER_RVA), weather_offset)
}
