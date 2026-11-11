pub(super) fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    let found = bytes.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_le_bytes([found[0], found[1]]))
}

pub(super) fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let found = bytes.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes([found[0], found[1], found[2], found[3]]))
}

pub(super) fn f32_at(bytes: &[u8], offset: usize) -> Option<f32> {
    u32_at(bytes, offset).map(f32::from_bits)
}
