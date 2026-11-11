use replicant_game::offsets::{
    ENTITY_RVA, ENTITY_ZONE_OFFSET, MAP_MANAGER_ACTIVE_CONTEXT_POINTER_OFFSET, MAP_MANAGER_RVA,
};
use replicant_game::{GameError, MemoryError, Replicant};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const LONGEST_PHASE_NAME: &str = "B_SEID_5000_S0020_a0080";

#[test]
fn current_phase_reads_a_name_longer_than_the_old_zone_window() {
    let mut memory = FakeMemory::new();
    let mut buffer = [0_u8; 0x20];
    buffer[..LONGEST_PHASE_NAME.len()].copy_from_slice(LONGEST_PHASE_NAME.as_bytes());
    memory.put_at_rva(ENTITY_RVA + ENTITY_ZONE_OFFSET, &buffer);
    assert_eq!(
        Replicant::new(memory).current_phase().as_deref(),
        Ok(LONGEST_PHASE_NAME)
    );
}

#[test]
fn map_manager_is_active_only_with_a_context_pointer() {
    let pointer_rva = MAP_MANAGER_RVA + MAP_MANAGER_ACTIVE_CONTEXT_POINTER_OFFSET as u32;
    let mut memory = FakeMemory::new();
    memory.put_at_rva(pointer_rva, &0_u64.to_le_bytes());
    assert_eq!(Replicant::new(memory).map_manager_is_active(), Ok(false));

    let mut memory = FakeMemory::new();
    memory.put_at_rva(pointer_rva, &0x2_0000_0000_u64.to_le_bytes());
    assert_eq!(Replicant::new(memory).map_manager_is_active(), Ok(true));
}

#[test]
fn map_manager_is_active_reports_unreadable_memory() {
    let game = Replicant::new(FakeMemory::new());
    let pointer_address =
        FAKE_MODULE_BASE + u64::from(MAP_MANAGER_RVA) + MAP_MANAGER_ACTIVE_CONTEXT_POINTER_OFFSET;
    assert_eq!(
        game.map_manager_is_active(),
        Err(GameError::Memory(MemoryError::Unreadable {
            address: pointer_address
        }))
    );
}
