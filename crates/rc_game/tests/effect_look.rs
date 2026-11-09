use rc_game::effects::{
    EFFECT_LOOK_RETRY_FRAMES, LookAttempt, OriginNames, look_attempt, look_needs_write,
    read_effect_look, write_effect_look,
};
use rc_game::offsets::{
    EFFECT_HANDLE_GENERATION_SHIFT, EFFECT_INSTANCE_CHANGED_FLAG_OFFSET,
    EFFECT_INSTANCE_EXTRA_OFFSET, EFFECT_INSTANCE_STYLE_OFFSET, EFFECT_POOL_GENERATION_OFFSET,
    EFFECT_POOL_RVA, EFFECT_POOL_SLOT_STRIDE,
};
use rc_proto::EffectLook;

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const INSTANCE: u64 = 0x2_2000_0000;
const POOL_INDEX: u32 = 3;
const GENERATION: u32 = 7;

fn look(alpha: f32) -> EffectLook {
    EffectLook {
        style: [0.9, 0.2, 0.1, alpha, 2.0, 2.0, 2.0, 2.0],
        extra: 1.5,
        flag: 1,
    }
}

fn handle() -> u32 {
    (GENERATION << EFFECT_HANDLE_GENERATION_SHIFT) | POOL_INDEX
}

fn memory_with_effect() -> FakeMemory {
    let mut memory = FakeMemory::new();
    let slot = FAKE_MODULE_BASE
        + u64::from(EFFECT_POOL_RVA)
        + u64::from(POOL_INDEX) * EFFECT_POOL_SLOT_STRIDE;
    memory.put(slot, &INSTANCE.to_le_bytes());
    memory.put(
        slot + EFFECT_POOL_GENERATION_OFFSET,
        &GENERATION.to_le_bytes(),
    );
    memory.put(INSTANCE + EFFECT_INSTANCE_STYLE_OFFSET, &[0; 32]);
    memory.put(INSTANCE + EFFECT_INSTANCE_EXTRA_OFFSET, &[0; 4]);
    memory.put(INSTANCE + EFFECT_INSTANCE_CHANGED_FLAG_OFFSET, &[0]);
    memory
}

#[test]
fn a_look_written_to_an_effect_reads_back_the_same() {
    let mut memory = memory_with_effect();
    assert!(write_effect_look(&mut memory, handle(), &look(1.0)).unwrap());
    assert_eq!(read_effect_look(&memory, handle()).unwrap(), Some(look(1.0)));
}

#[test]
fn a_gone_effect_has_no_look_and_takes_none() {
    let mut memory = memory_with_effect();
    let stale = ((GENERATION + 1) << EFFECT_HANDLE_GENERATION_SHIFT) | POOL_INDEX;
    assert_eq!(read_effect_look(&memory, stale).unwrap(), None);
    assert!(!write_effect_look(&mut memory, stale, &look(1.0)).unwrap());
}

#[test]
fn a_look_is_written_only_when_the_sender_set_it_and_it_differs() {
    assert!(look_needs_write(&look(1.0), &look(0.0)));
    assert!(!look_needs_write(&look(1.0), &look(1.0)));
    assert!(!look_needs_write(&look(0.0), &look(1.0)));
    let mut other_color = look(1.0);
    other_color.style[0] = 0.5;
    assert!(look_needs_write(&look(1.0), &other_color));
}

#[test]
fn the_look_wait_ends_after_the_retry_limit() {
    assert_eq!(look_attempt(0, true), LookAttempt::Use);
    assert_eq!(look_attempt(EFFECT_LOOK_RETRY_FRAMES, true), LookAttempt::Use);
    assert_eq!(look_attempt(0, false), LookAttempt::Wait);
    assert_eq!(
        look_attempt(EFFECT_LOOK_RETRY_FRAMES - 1, false),
        LookAttempt::Wait
    );
    assert_eq!(
        look_attempt(EFFECT_LOOK_RETRY_FRAMES, false),
        LookAttempt::GiveUp
    );
}

#[test]
fn an_origin_is_logged_once_per_name_and_caller() {
    let mut origins = OriginNames::default();
    assert!(origins.is_first("hitBlood02", 0x32_1989));
    assert!(!origins.is_first("hitBlood02", 0x32_1989));
    assert!(origins.is_first("hitBlood02", 0x32_1990));
    assert!(origins.is_first("spark01", 0x32_1989));
}
