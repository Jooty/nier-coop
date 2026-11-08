use rc_proto::MAX_STORY_CALL_NAME_BYTES;

use crate::dummy_hits::DUMMY_CALLBACK_NAME_ARG_INDEX;
use crate::story_jump::{
    NATIVE_ARG_FIRST_OFFSET_BYTES, NATIVE_ARG_STRIDE_BYTES, NATIVE_ARGS_HEADER_BYTES,
};
use crate::story_relay::{ARG_TAG_TEXT, ARG_TEXT_CAPACITY_BYTES, ARG_TEXT_OFFSET};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DummyNameRefusal {
    Truncated,
    NotText(i32),
    Unterminated,
    Empty,
    TooLong,
    NotUtf8,
}

pub const fn dummy_native_args_len() -> usize {
    native_text_args_len(DUMMY_CALLBACK_NAME_ARG_INDEX)
}

pub const fn native_text_args_len(index: usize) -> usize {
    native_arg_slot(index) + ARG_TEXT_OFFSET + ARG_TEXT_CAPACITY_BYTES
}

pub fn dummy_callback_name(args: &[u8]) -> Result<String, DummyNameRefusal> {
    let name = native_text_arg(args, DUMMY_CALLBACK_NAME_ARG_INDEX)?;
    if name.len() > MAX_STORY_CALL_NAME_BYTES {
        return Err(DummyNameRefusal::TooLong);
    }
    Ok(name)
}

pub fn native_text_arg(args: &[u8], index: usize) -> Result<String, DummyNameRefusal> {
    let slot = native_arg_slot(index);
    let tag_bytes = args
        .get(slot..slot + 4)
        .ok_or(DummyNameRefusal::Truncated)?;
    let tag = i32::from_le_bytes([tag_bytes[0], tag_bytes[1], tag_bytes[2], tag_bytes[3]]);
    if tag != ARG_TAG_TEXT {
        return Err(DummyNameRefusal::NotText(tag));
    }
    let text = args
        .get(slot + ARG_TEXT_OFFSET..slot + ARG_TEXT_OFFSET + ARG_TEXT_CAPACITY_BYTES)
        .ok_or(DummyNameRefusal::Truncated)?;
    let end = text
        .iter()
        .position(|&byte| byte == 0)
        .ok_or(DummyNameRefusal::Unterminated)?;
    if end == 0 {
        return Err(DummyNameRefusal::Empty);
    }
    String::from_utf8(text[..end].to_vec()).map_err(|_| DummyNameRefusal::NotUtf8)
}

const fn native_arg_slot(index: usize) -> usize {
    NATIVE_ARGS_HEADER_BYTES + index * NATIVE_ARG_STRIDE_BYTES + NATIVE_ARG_FIRST_OFFSET_BYTES
}
