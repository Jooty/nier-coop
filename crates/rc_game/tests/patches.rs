use rc_game::offsets::{INFINITE_MAGIC_ORIGINAL_BYTES, INFINITE_MAGIC_PATCH_RVA};
use rc_game::{GameError, Replicant};

use crate::fake_memory::FakeMemory;

fn memory_with_original_code() -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(INFINITE_MAGIC_PATCH_RVA, &INFINITE_MAGIC_ORIGINAL_BYTES);
    memory
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
    memory.put_at_rva(INFINITE_MAGIC_PATCH_RVA, &[0xCC; 6]);
    let mut game = Replicant::new(memory);
    let result = game.set_infinite_magic(true);
    assert_eq!(
        result,
        Err(GameError::UnexpectedBytes {
            rva: INFINITE_MAGIC_PATCH_RVA
        })
    );
    assert_eq!(
        game.memory().get_at_rva(INFINITE_MAGIC_PATCH_RVA, 6),
        [0xCC; 6]
    );
}

#[test]
fn patch_state_follows_the_toggles() {
    let mut game = Replicant::new(memory_with_original_code());
    assert_eq!(game.infinite_magic_enabled(), Ok(false));
    game.set_infinite_magic(true).expect("code is writable");
    assert_eq!(game.infinite_magic_enabled(), Ok(true));
}
