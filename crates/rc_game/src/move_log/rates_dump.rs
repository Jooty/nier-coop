use crate::memory::GameMemory;
use crate::move_log::{
    MOVE_LOG_RATE_CHANNEL_COUNT, MOVE_LOG_RATE_ID_COUNT, RATE_TABLE_CHANNEL_STRIDE,
    RATE_TABLE_ID_STRIDE, RATE_TABLE_OFFSET, STATE_161F0_OFFSET,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RatesDump {
    pub state_161f0: Option<i32>,
    pub slots: Option<Vec<Vec<f32>>>,
}

pub fn read_rates_dump<M: GameMemory>(memory: &M, actor: u64) -> RatesDump {
    let mut table = vec![0_u8; MOVE_LOG_RATE_ID_COUNT * RATE_TABLE_ID_STRIDE];
    let slots = memory
        .read(actor + RATE_TABLE_OFFSET, &mut table)
        .ok()
        .map(|()| {
            (0..MOVE_LOG_RATE_ID_COUNT)
                .map(|id| {
                    (0..MOVE_LOG_RATE_CHANNEL_COUNT)
                        .map(|channel| {
                            let at = id * RATE_TABLE_ID_STRIDE + channel * RATE_TABLE_CHANNEL_STRIDE;
                            f32::from_le_bytes([table[at], table[at + 1], table[at + 2], table[at + 3]])
                        })
                        .collect()
                })
                .collect()
        });
    RatesDump {
        state_161f0: memory.read_i32(actor + STATE_161F0_OFFSET).ok(),
        slots,
    }
}

pub fn format_rates_dump(role: &str, when: &str, dump: &RatesDump) -> Vec<String> {
    let state = dump
        .state_161f0
        .map_or_else(|| "?".to_owned(), |state| state.to_string());
    let mut lines = vec![format!(
        "move rates: role {role}, when {when}, state161f0={state}"
    )];
    let Some(slots) = &dump.slots else {
        lines.push("move rate table unreadable".to_owned());
        return lines;
    };
    for (id, channels) in slots.iter().enumerate() {
        let values: Vec<String> = channels
            .iter()
            .enumerate()
            .map(|(channel, value)| format!("{id}:{channel}={value:.2}"))
            .collect();
        lines.push(format!("move rate {id}: {}", values.join(" ")));
    }
    lines
}
