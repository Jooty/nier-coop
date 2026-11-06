use crate::party_frame::{
    PACK_MAGIC, PACK_RESOURCE_START_FIELD, TEXTURE_FORMAT_FROM_TYPE_NAME, TEXTURE_HEAD_TYPE_NAME,
    TEXTURE_HEIGHT_FROM_TYPE_NAME, TEXTURE_WIDTH_FROM_TYPE_NAME,
};

pub(super) struct TextureHead {
    pub width: u32,
    pub height: u32,
    pub format: u32,
}

pub(super) fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes(slice.try_into().ok()?))
}

pub(super) fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_le_bytes(slice.try_into().ok()?))
}

pub(super) fn read_f32(bytes: &[u8], offset: usize) -> Option<f32> {
    read_u32(bytes, offset).map(f32::from_bits)
}

pub(super) fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub(super) fn pack_resource_start(bytes: &[u8]) -> Option<usize> {
    if bytes.get(..4)? != PACK_MAGIC {
        return None;
    }
    read_u32(bytes, PACK_RESOURCE_START_FIELD).map(|start| start as usize)
}

pub(super) fn first_texture_head(bytes: &[u8]) -> Option<TextureHead> {
    let at = find_bytes(bytes, TEXTURE_HEAD_TYPE_NAME)?;
    Some(TextureHead {
        width: read_u32(bytes, at + TEXTURE_WIDTH_FROM_TYPE_NAME)?,
        height: read_u32(bytes, at + TEXTURE_HEIGHT_FROM_TYPE_NAME)?,
        format: read_u32(bytes, at + TEXTURE_FORMAT_FROM_TYPE_NAME)?,
    })
}
