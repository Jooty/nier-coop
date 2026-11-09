use rc_proto::PartnerBody;

use crate::offsets::PUPPET_FALLBACK_CLASS_ID;

pub const BOOK_BOB_PERIOD_MS: u64 = 2400;
pub const BOOK_HOVER_AMPLITUDE: f32 = 0.06;
const BOOK_SIDE_OFFSET: f32 = 0.7;
const BOOK_HEAD_HEIGHT: f32 = 1.5;
const BOOK_BEHIND_OFFSET: f32 = -0.5;
const PROLOGUE_BOOK_PACK: &str = "bookb012";
const DEFAULT_BOOK_PACK: &str = "bookw011";

pub fn book_pack_name(body: PartnerBody) -> &'static str {
    match body {
        PartnerBody::PrologueCharacter => PROLOGUE_BOOK_PACK,
        _ => DEFAULT_BOOK_PACK,
    }
}

/// Offset from the body's root in the body's own axes (x beside, y up, z forward).
pub fn book_hover_offset(time_ms: u64) -> [f32; 3] {
    let phase = (time_ms % BOOK_BOB_PERIOD_MS) as f32 / BOOK_BOB_PERIOD_MS as f32;
    let bob = (phase * core::f32::consts::TAU).sin() * BOOK_HOVER_AMPLITUDE;
    [BOOK_SIDE_OFFSET, BOOK_HEAD_HEIGHT + bob, BOOK_BEHIND_OFFSET]
}

pub fn book_class_id(local_book_class: Option<i32>) -> i32 {
    local_book_class
        .filter(|class| *class > 0)
        .unwrap_or(PUPPET_FALLBACK_CLASS_ID)
}
