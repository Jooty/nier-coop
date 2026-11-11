use automata_game::address::{
    GENERATOR_LIST_HEAD_VALUE, GENERATOR_SLOT_ADD_FUNCTION, GENERATOR_SLOT_FREE_FUNCTION,
};
use automata_game::enemies::{
    EnemySpawn, GENERATOR_HEADER_LEN, GENERATOR_SET_LEN, GENERATOR_SLOT_ARRAY_ADD_VTABLE_OFFSET,
    GENERATOR_SLOT_ARRAY_OFFSET, GENERATOR_SLOT_LEN, GeneratorHeader, GeneratorSlot,
    MOST_GENERATORS_WALKED, MOST_SETS_OF_ONE_GENERATOR, MOST_SLOTS_READ_OF_ONE_GENERATOR,
    generator_header, generator_slots, new_slot_bytes, object_id_of_set,
};

use crate::game_calls::GameCalls;

type SlotAdd = unsafe extern "system" fn(usize, *const NewSlot) -> u8;
type SlotFree = unsafe extern "system" fn(usize);

#[repr(C, align(16))]
struct NewSlot([u8; GENERATOR_SLOT_LEN]);

impl GameCalls {
    /// Every enemy generator of the game's own list, with its address. Reads only.
    pub fn read_generators(&self) -> Vec<(usize, GeneratorHeader)> {
        let mut generators = Vec::new();
        let mut at = self.read_u64(GENERATOR_LIST_HEAD_VALUE).unwrap_or(0);
        while at != 0 && generators.len() < MOST_GENERATORS_WALKED {
            let Some(header) = self.read_generator(at as usize) else {
                break;
            };
            generators.push((at as usize, header));
            at = header.next;
        }
        generators
    }

    pub fn read_generator(&self, generator: usize) -> Option<GeneratorHeader> {
        self.read_bytes_at(generator as u64, GENERATOR_HEADER_LEN)
            .ok()
            .and_then(|bytes| generator_header(&bytes))
    }

    pub fn read_generator_slots(&self, header: &GeneratorHeader) -> Vec<GeneratorSlot> {
        if header.slots == 0 || !(1..=MOST_SLOTS_READ_OF_ONE_GENERATOR).contains(&header.slot_count)
        {
            return Vec::new();
        }
        self.read_bytes_at(
            header.slots,
            header.slot_count as usize * GENERATOR_SLOT_LEN,
        )
        .map(|bytes| generator_slots(&bytes))
        .unwrap_or_default()
    }

    pub fn object_id_of_generator_set(
        &self,
        header: &GeneratorHeader,
        set_index: u32,
    ) -> Option<u32> {
        if header.sets == 0
            || set_index >= MOST_SETS_OF_ONE_GENERATOR
            || u64::from(set_index) >= header.set_count
        {
            return None;
        }
        let bytes = self
            .read_bytes_at(
                header.sets + u64::from(set_index) * GENERATOR_SET_LEN as u64,
                GENERATOR_SET_LEN,
            )
            .ok()?;
        object_id_of_set(&bytes, 0)
    }

    /// Adds a slot as the game's own spawn decision does; false when the list or the game refuses.
    /// # Safety
    /// `generator` must be a live enemy generator, called from inside the game's update of it.
    #[expect(
        unsafe_code,
        reason = "calls the add function of a generator's slot list"
    )]
    pub unsafe fn add_generator_slot(&self, generator: usize, spawn: &EnemySpawn) -> bool {
        let list = generator + GENERATOR_SLOT_ARRAY_OFFSET;
        let add = self.address_value(GENERATOR_SLOT_ADD_FUNCTION);
        let list_adds_with_the_noted_function = self
            .read_u64_at(list as u64)
            .and_then(|vtable| {
                self.read_u64_at(vtable + GENERATOR_SLOT_ARRAY_ADD_VTABLE_OFFSET as u64)
            })
            .is_ok_and(|found| found == add);
        if !list_adds_with_the_noted_function {
            return false;
        }
        let record = NewSlot(new_slot_bytes(
            spawn.set_index,
            &spawn.position,
            &spawn.rotation,
        ));
        // SAFETY: the function is the one the list's own vtable names for adding, checked above;
        // it copies the 0x40-byte record, which lives and is aligned for the call.
        unsafe { core::mem::transmute::<usize, SlotAdd>(add as usize)(list, &record) != 0 }
    }

    /// What the game does to a slot whose enemy is gone; the next update takes the slot out.
    /// # Safety
    /// `slot` must be a slot of a live generator, called from inside the game's update of it.
    #[expect(
        unsafe_code,
        reason = "calls the game's slot free function of a generator"
    )]
    pub unsafe fn free_generator_slot(&self, slot: usize) {
        // SAFETY: the address is the slot free function of the verified build; the caller
        // upholds the contract above.
        unsafe {
            core::mem::transmute::<usize, SlotFree>(self.address_of(GENERATOR_SLOT_FREE_FUNCTION))(
                slot,
            );
        }
    }
}
