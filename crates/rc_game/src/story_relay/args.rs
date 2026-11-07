use rc_proto::{MAX_STORY_CALL_ARGS, MAX_STORY_CALL_TEXT_BYTES, StoryCallArg};

use crate::story_progress::fnv_hash;
use crate::story_relay::{
    ARG_STRIDE_BYTES, ARG_TAG_INT, ARG_TAG_NUMBER, ARG_TAG_POINTER, ARG_TAG_TEXT,
    ARG_TEXT_CAPACITY_BYTES, ARG_TEXT_OFFSET, ARG_VALUE_OFFSET, ARGS_HEADER_BYTES,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgsRefusal {
    TooMany,
    Pointer,
    MachineLocalNumber,
    UnknownTag,
    TextTooLong,
    Truncated,
}

pub fn args_block_len(count: i32) -> Option<usize> {
    let count = usize::try_from(count).ok()?;
    (count <= MAX_STORY_CALL_ARGS).then_some(ARGS_HEADER_BYTES + count * ARG_STRIDE_BYTES)
}

pub fn read_args_block(bytes: &[u8]) -> Result<Vec<StoryCallArg>, ArgsRefusal> {
    let count = i32::from_le_bytes(
        bytes
            .get(..4)
            .and_then(|head| head.try_into().ok())
            .ok_or(ArgsRefusal::Truncated)?,
    );
    let len = args_block_len(count).ok_or(ArgsRefusal::TooMany)?;
    let block = bytes.get(..len).ok_or(ArgsRefusal::Truncated)?;
    block[ARGS_HEADER_BYTES..]
        .as_chunks::<ARG_STRIDE_BYTES>()
        .0
        .iter()
        .map(read_one_arg)
        .collect()
}

pub fn write_args_block(args: &[StoryCallArg]) -> Vec<u8> {
    let mut block = vec![0_u8; ARGS_HEADER_BYTES + args.len() * ARG_STRIDE_BYTES];
    block[..4].copy_from_slice(&(args.len() as i32).to_le_bytes());
    for (index, arg) in args.iter().enumerate() {
        let start = ARGS_HEADER_BYTES + index * ARG_STRIDE_BYTES;
        let slot = &mut block[start..start + ARG_STRIDE_BYTES];
        match arg {
            StoryCallArg::Number(value) => {
                slot[..4].copy_from_slice(&ARG_TAG_NUMBER.to_le_bytes());
                slot[ARG_VALUE_OFFSET..ARG_VALUE_OFFSET + 8].copy_from_slice(&value.to_le_bytes());
            }
            StoryCallArg::Int(value) => {
                slot[..4].copy_from_slice(&ARG_TAG_INT.to_le_bytes());
                slot[ARG_VALUE_OFFSET..ARG_VALUE_OFFSET + 4].copy_from_slice(&value.to_le_bytes());
            }
            StoryCallArg::Text(text) => {
                slot[..4].copy_from_slice(&ARG_TAG_TEXT.to_le_bytes());
                slot[ARG_TEXT_OFFSET..ARG_TEXT_OFFSET + text.len()].copy_from_slice(text);
            }
        }
    }
    block
}

pub fn args_hash(args: &[StoryCallArg]) -> u64 {
    let mut canonical = Vec::new();
    for arg in args {
        match arg {
            StoryCallArg::Number(value) => {
                canonical.push(1);
                canonical.extend_from_slice(&value.to_le_bytes());
            }
            StoryCallArg::Int(value) => {
                canonical.push(2);
                canonical.extend_from_slice(&value.to_le_bytes());
            }
            StoryCallArg::Text(text) => {
                canonical.push(3);
                canonical.extend_from_slice(text);
                canonical.push(0);
            }
        }
    }
    fnv_hash(&canonical)
}

fn read_one_arg(slot: &[u8; ARG_STRIDE_BYTES]) -> Result<StoryCallArg, ArgsRefusal> {
    let tag = i32::from_le_bytes([slot[0], slot[1], slot[2], slot[3]]);
    match tag {
        ARG_TAG_NUMBER => {
            let value = slot[ARG_VALUE_OFFSET..ARG_VALUE_OFFSET + 8]
                .try_into()
                .map_err(|_| ArgsRefusal::Truncated)?;
            let bits = u64::from_le_bytes(value);
            let number = f64::from_bits(bits);
            if !number.is_finite() || number.is_subnormal() {
                return Err(ArgsRefusal::MachineLocalNumber);
            }
            Ok(StoryCallArg::Number(bits))
        }
        ARG_TAG_INT => {
            let value = slot[ARG_VALUE_OFFSET..ARG_VALUE_OFFSET + 4]
                .try_into()
                .map_err(|_| ArgsRefusal::Truncated)?;
            Ok(StoryCallArg::Int(i32::from_le_bytes(value)))
        }
        ARG_TAG_TEXT => {
            let buffer = &slot[ARG_TEXT_OFFSET..ARG_TEXT_OFFSET + ARG_TEXT_CAPACITY_BYTES];
            let end = buffer
                .iter()
                .position(|&byte| byte == 0)
                .ok_or(ArgsRefusal::TextTooLong)?;
            if end > MAX_STORY_CALL_TEXT_BYTES {
                return Err(ArgsRefusal::TextTooLong);
            }
            Ok(StoryCallArg::Text(buffer[..end].to_vec()))
        }
        ARG_TAG_POINTER => Err(ArgsRefusal::Pointer),
        _ => Err(ArgsRefusal::UnknownTag),
    }
}
