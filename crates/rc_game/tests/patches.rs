use rc_game::offsets::{
    INFINITE_HEALTH_ORIGINAL_BYTES, INFINITE_HEALTH_PATCH_RVA, INFINITE_MAGIC_ORIGINAL_BYTES,
    INFINITE_MAGIC_PATCH_RVA,
};
use rc_game::{GameError, Replicant};

use crate::fake_memory::FakeMemory;

fn memory_with_original_code() -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(INFINITE_HEALTH_PATCH_RVA, &INFINITE_HEALTH_ORIGINAL_BYTES);
    memory.put_at_rva(INFINITE_MAGIC_PATCH_RVA, &INFINITE_MAGIC_ORIGINAL_BYTES);
    memory
}

#[test]
fn infinite_health_nops_five_bytes_and_restores_them() {
    let mut game = Replicant::new(memory_with_original_code());
    game.set_infinite_health(true).expect("code is writable");
    assert_eq!(
        game.memory().get_at_rva(INFINITE_HEALTH_PATCH_RVA, 5),
        [0x90; 5]
    );
    game.set_infinite_health(false).expect("code is writable");
    assert_eq!(
        game.memory().get_at_rva(INFINITE_HEALTH_PATCH_RVA, 5),
        INFINITE_HEALTH_ORIGINAL_BYTES
    );
}

#[test]
fn infinite_magic_nops_six_bytes() {
    let mut game = Replicant::new(memory_with_original_code());
    game.set_infinite_magic(true).expect("code is writable");
    assert_eq!(
        game.memory().get_at_rva(INFINITE_MAGIC_PATCH_RVA, 6),
        [0x90; 6]
    );
}

#[test]
fn a_patch_over_unknown_code_is_refused() {
    let mut memory = memory_with_original_code();
    memory.put_at_rva(INFINITE_HEALTH_PATCH_RVA, &[0xCC; 5]);
    let mut game = Replicant::new(memory);
    let result = game.set_infinite_health(true);
    assert_eq!(
        result,
        Err(GameError::UnexpectedBytes {
            rva: INFINITE_HEALTH_PATCH_RVA
        })
    );
    assert_eq!(
        game.memory().get_at_rva(INFINITE_HEALTH_PATCH_RVA, 5),
        [0xCC; 5]
    );
}

#[test]
fn patch_state_follows_the_toggles() {
    let mut game = Replicant::new(memory_with_original_code());
    assert_eq!(game.infinite_health_enabled(), Ok(false));
    game.set_infinite_health(true).expect("code is writable");
    assert_eq!(game.infinite_health_enabled(), Ok(true));
    assert_eq!(game.infinite_magic_enabled(), Ok(false));
    game.set_infinite_magic(true).expect("code is writable");
    assert_eq!(game.infinite_magic_enabled(), Ok(true));
}
