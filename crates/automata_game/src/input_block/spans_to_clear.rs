use crate::address::PAD_INPUT_RECORDS;
use crate::exe::ExeVariant;
use crate::input_block::{ClearSpan, PAD_RECORD_COUNT, PAD_RECORD_STRIDE};

pub fn spans_to_clear(menu_open: bool, variant: ExeVariant) -> Vec<ClearSpan> {
    if !menu_open {
        return Vec::new();
    }
    vec![ClearSpan {
        rva: PAD_INPUT_RECORDS.rva(variant),
        len: PAD_RECORD_COUNT * PAD_RECORD_STRIDE,
    }]
}
