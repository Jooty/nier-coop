use core::ops::Range;

use replicant_proto::MAX_STORY_CALL_NAME_BYTES;

use crate::dummy_hits::{native_arg_tag_range, native_arg_text_range, native_text_from_parts};
use crate::message_handlers::{
    SET_MESSAGE_CALLBACK_MESSAGE_ID_ARG_INDEX, SET_MESSAGE_CALLBACK_NAME_ARG_INDEX,
};
use crate::story_jump::{
    NATIVE_ARG_INTEGER_TYPE_TAG, NATIVE_ARG_REAL_TYPE_TAG, NATIVE_ARG_VALUE_OFFSET_BYTES,
};

pub const fn message_name_tag_range() -> Range<usize> {
    native_arg_tag_range(SET_MESSAGE_CALLBACK_NAME_ARG_INDEX)
}

pub const fn message_name_text_range() -> Range<usize> {
    native_arg_text_range(SET_MESSAGE_CALLBACK_NAME_ARG_INDEX)
}

pub const fn message_id_tag_range() -> Range<usize> {
    native_arg_tag_range(SET_MESSAGE_CALLBACK_MESSAGE_ID_ARG_INDEX)
}

pub const fn message_id_value_range() -> Range<usize> {
    let tag = message_id_tag_range();
    tag.start + NATIVE_ARG_VALUE_OFFSET_BYTES..tag.start + NATIVE_ARG_VALUE_OFFSET_BYTES + 8
}

pub fn handler_name_from_parts(tag: &[u8], text: &[u8]) -> Option<String> {
    let name = native_text_from_parts(tag, text).ok()?;
    let valid =
        name.len() <= MAX_STORY_CALL_NAME_BYTES && name.bytes().all(|byte| byte.is_ascii_graphic());
    valid.then_some(name)
}

pub fn message_id_from_parts(tag: &[u8], value: &[u8]) -> Option<i32> {
    let tag = i32::from_le_bytes(tag.try_into().ok()?);
    if u32::try_from(tag).ok()? == NATIVE_ARG_INTEGER_TYPE_TAG {
        let bytes: [u8; 4] = value.get(..4)?.try_into().ok()?;
        return Some(i32::from_le_bytes(bytes));
    }
    if u32::try_from(tag).ok()? == NATIVE_ARG_REAL_TYPE_TAG {
        let bytes: [u8; 8] = value.get(..8)?.try_into().ok()?;
        let number = f64::from_le_bytes(bytes);
        let in_range =
            number.is_finite() && number.fract() == 0.0 && number.abs() < f64::from(i32::MAX);
        return in_range.then_some(number as i32);
    }
    None
}
