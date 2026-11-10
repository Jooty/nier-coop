const HALF_WORD_BITS: u32 = 16;
const HALF_WORD_MASK: u32 = 0xFFFF;

pub fn pack_hit_kind(kind: i32, source_class: i32) -> u32 {
    (source_class.cast_unsigned() & HALF_WORD_MASK) << HALF_WORD_BITS
        | (kind.cast_unsigned() & HALF_WORD_MASK)
}

pub fn unpack_hit_kind(packed: u32) -> (i32, Option<i32>) {
    let kind = i32::from((packed & HALF_WORD_MASK) as u16 as i16);
    let class = i32::from((packed >> HALF_WORD_BITS) as u16 as i16);
    (kind, (class != 0).then_some(class))
}
