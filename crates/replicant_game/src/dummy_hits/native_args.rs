use core::ops::Range;

use replicant_proto::MAX_STORY_CALL_NAME_BYTES;

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
    let tag = args
        .get(native_arg_tag_range(index))
        .ok_or(DummyNameRefusal::Truncated)?;
    let text = args
        .get(native_arg_text_range(index))
        .ok_or(DummyNameRefusal::Truncated)?;
    native_text_from_parts(tag, text)
}

pub const fn native_arg_tag_range(index: usize) -> Range<usize> {
    let slot = native_arg_slot(index);
    slot..slot + 4
}

pub const fn native_arg_text_range(index: usize) -> Range<usize> {
    let slot = native_arg_slot(index);
    slot + ARG_TEXT_OFFSET..slot + ARG_TEXT_OFFSET + ARG_TEXT_CAPACITY_BYTES
}

pub fn native_text_from_parts(tag_bytes: &[u8], text: &[u8]) -> Result<String, DummyNameRefusal> {
    let [b0, b1, b2, b3] = *tag_bytes else {
        return Err(DummyNameRefusal::Truncated);
    };
    let tag = i32::from_le_bytes([b0, b1, b2, b3]);
    if tag != ARG_TAG_TEXT {
        return Err(DummyNameRefusal::NotText(tag));
    }
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
