use crate::memory::GameMemory;
use crate::move_log::{
    GLOBAL_SCALE_FIRST_FIELD_OFFSET, GLOBAL_SCALE_FUNCTION_BYTE_COUNT, GLOBAL_SCALE_FUNCTION_RVA,
    GLOBAL_SCALE_SECOND_FIELD_OFFSET, RATE_PAIR_COUNT, RATE_PAIR_FIRST_OFFSET,
    RATE_PAIR_SECOND_OFFSET, RATE_PAIR_STRIDE, RATE_PRODUCT_FIRST_INDEX, RATE_PRODUCT_LAST_INDEX,
    STATE_161F0_OFFSET,
};

const LOAD_RAX_FROM_RIP_PREFIX: [u8; 3] = [0x48, 0x8B, 0x05];
const LEA_RAX_FROM_RIP_PREFIX: [u8; 3] = [0x48, 0x8D, 0x05];
const RIP_RELATIVE_INSTRUCTION_LENGTH: u64 = 7;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RatesDump {
    pub state_161f0: Option<i32>,
    pub pairs: Vec<Option<[f32; 4]>>,
    pub global_pointer: Option<u64>,
    pub global_fields: Option<[f32; 2]>,
    pub global_function_bytes: Option<Vec<u8>>,
}

pub fn read_rates_dump<M: GameMemory>(memory: &M, actor: u64) -> RatesDump {
    let function = memory.module_base() + GLOBAL_SCALE_FUNCTION_RVA;
    let function_bytes = vec_read(memory, function, GLOBAL_SCALE_FUNCTION_BYTE_COUNT);
    let global_pointer = function_bytes
        .as_deref()
        .and_then(|bytes| global_pointer_from_code(memory, function, bytes))
        .filter(|pointer| *pointer != 0);
    RatesDump {
        state_161f0: memory.read_i32(actor + STATE_161F0_OFFSET).ok(),
        pairs: (0..RATE_PAIR_COUNT)
            .map(|k| read_pairs(memory, actor, k))
            .collect(),
        global_pointer,
        global_fields: global_pointer.and_then(|pointer| {
            Some([
                memory
                    .read_f32(pointer + GLOBAL_SCALE_FIRST_FIELD_OFFSET)
                    .ok()?,
                memory
                    .read_f32(pointer + GLOBAL_SCALE_SECOND_FIELD_OFFSET)
                    .ok()?,
            ])
        }),
        global_function_bytes: function_bytes,
    }
}

fn vec_read<M: GameMemory>(memory: &M, address: u64, count: usize) -> Option<Vec<u8>> {
    let mut bytes = vec![0_u8; count];
    memory.read(address, &mut bytes).ok()?;
    Some(bytes)
}

fn global_pointer_from_code<M: GameMemory>(memory: &M, function: u64, code: &[u8]) -> Option<u64> {
    let displacement = i32::from_le_bytes(code.get(3..7)?.try_into().ok()?);
    let target = (function + RIP_RELATIVE_INSTRUCTION_LENGTH).wrapping_add_signed(i64::from(displacement));
    match code.get(..3)? {
        prefix if prefix == LOAD_RAX_FROM_RIP_PREFIX => memory.read_u64(target).ok(),
        prefix if prefix == LEA_RAX_FROM_RIP_PREFIX => Some(target),
        _ => None,
    }
}

fn read_pairs<M: GameMemory>(memory: &M, actor: u64, k: u64) -> Option<[f32; 4]> {
    let first = actor + RATE_PAIR_FIRST_OFFSET + k * RATE_PAIR_STRIDE;
    let second = actor + RATE_PAIR_SECOND_OFFSET + k * RATE_PAIR_STRIDE;
    Some([
        memory.read_f32(first).ok()?,
        memory.read_f32(first + 4).ok()?,
        memory.read_f32(second).ok()?,
        memory.read_f32(second + 4).ok()?,
    ])
}

pub fn format_rates_dump(role: &str, when: &str, dump: &RatesDump) -> Vec<String> {
    let state = dump
        .state_161f0
        .map_or_else(|| "?".to_owned(), |state| state.to_string());
    let pointer = dump
        .global_pointer
        .map_or_else(|| "?".to_owned(), |pointer| format!("{pointer:#x}"));
    let fields = dump.global_fields.map_or_else(
        || "?".to_owned(),
        |[a, b]| format!("{a:.4}*{b:.4}={:.4}", a * b),
    );
    let code = dump.global_function_bytes.as_deref().map_or_else(
        || "?".to_owned(),
        |bytes| {
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        },
    );
    let mut lines = vec![format!(
        "move rates: role {role}, when {when}, state161f0={state}, global_scale ptr={pointer} \
         +0x48*+0x18={fields}, scale_fn_code={code}"
    )];
    for (index, pair) in dump.pairs.iter().enumerate() {
        let text = pair.map_or_else(
            || "?".to_owned(),
            |[a, b, c, d]| {
                format!(
                    "{a:.4}/{b:.4} x {c:.4}/{d:.4}{}",
                    product_suffix(index, [a, b, c, d])
                )
            },
        );
        lines.push(format!("move rate {index}: {text}"));
    }
    lines
}

fn product_suffix(index: usize, values: [f32; 4]) -> String {
    if (RATE_PRODUCT_FIRST_INDEX..=RATE_PRODUCT_LAST_INDEX).contains(&index) {
        format!(" = {:.4}", values.iter().product::<f32>())
    } else {
        String::new()
    }
}
