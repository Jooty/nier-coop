use crate::hud_look::le_bytes::u32_at;
use crate::hud_look::{
    CRILAYLA_HEADER_BYTES, CRILAYLA_LARGEST_UNPACKED_BYTES, CRILAYLA_MAGIC,
    CRILAYLA_RAW_PREFIX_BYTES,
};

const COPY_SHORTEST_BYTES: usize = 3;
const COPY_OFFSET_BITS: u32 = 13;
const COPY_LENGTH_STEP_BITS: [u32; 3] = [2, 3, 5];
const COPY_LENGTH_TAIL_BITS: u32 = 8;

struct BitsFromTheEnd<'a> {
    bytes: &'a [u8],
    unread: usize,
    current: u8,
    bits_left: u32,
}

impl BitsFromTheEnd<'_> {
    fn take(&mut self, count: u32) -> Option<usize> {
        let mut value = 0_usize;
        for _ in 0..count {
            if self.bits_left == 0 {
                self.unread = self.unread.checked_sub(1)?;
                self.current = self.bytes[self.unread];
                self.bits_left = 8;
            }
            self.bits_left -= 1;
            value = (value << 1) | usize::from((self.current >> self.bits_left) & 1);
        }
        Some(value)
    }
}

/// The file a game archive stores packed; `None` when the bytes are not a whole packed file.
pub fn unpack_crilayla(packed: &[u8]) -> Option<Vec<u8>> {
    if !packed.starts_with(CRILAYLA_MAGIC) {
        return None;
    }
    let unpacked_size = u32_at(packed, 8)? as usize;
    let stream_size = u32_at(packed, 12)? as usize;
    if unpacked_size > CRILAYLA_LARGEST_UNPACKED_BYTES {
        return None;
    }
    let prefix_start = CRILAYLA_HEADER_BYTES.checked_add(stream_size)?;
    let prefix = packed.get(prefix_start..prefix_start + CRILAYLA_RAW_PREFIX_BYTES)?;
    let mut unpacked = vec![0_u8; unpacked_size + CRILAYLA_RAW_PREFIX_BYTES];
    unpacked[..CRILAYLA_RAW_PREFIX_BYTES].copy_from_slice(prefix);
    let stream = &packed[CRILAYLA_HEADER_BYTES..prefix_start];
    let mut bits = BitsFromTheEnd {
        bytes: stream,
        unread: stream.len(),
        current: 0,
        bits_left: 0,
    };
    // The stream is read from its end and the file is written from its end.
    let mut write = unpacked.len();
    while write > CRILAYLA_RAW_PREFIX_BYTES {
        if bits.take(1)? == 0 {
            write -= 1;
            unpacked[write] = bits.take(8)? as u8;
            continue;
        }
        let offset = bits.take(COPY_OFFSET_BITS)? + COPY_SHORTEST_BYTES;
        let length = copy_length(&mut bits)?;
        for _ in 0..length {
            if write == CRILAYLA_RAW_PREFIX_BYTES {
                break;
            }
            write -= 1;
            unpacked[write] = *unpacked.get(write + offset)?;
        }
    }
    Some(unpacked)
}

fn copy_length(bits: &mut BitsFromTheEnd<'_>) -> Option<usize> {
    let mut length = COPY_SHORTEST_BYTES;
    for step_bits in COPY_LENGTH_STEP_BITS {
        let step = bits.take(step_bits)?;
        length += step;
        if step != (1 << step_bits) - 1 {
            return Some(length);
        }
    }
    loop {
        let step = bits.take(COPY_LENGTH_TAIL_BITS)?;
        length += step;
        if step != (1 << COPY_LENGTH_TAIL_BITS) - 1 {
            return Some(length);
        }
    }
}
