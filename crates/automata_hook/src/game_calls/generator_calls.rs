use automata_game::address::{
    ENTITY_MANAGER_OBJECT, GENERATOR_GROUP_COMPONENT_KIND_VALUE, GENERATOR_LIST_HEAD_VALUE,
    GENERATOR_SLOT_ADD_FUNCTION, GENERATOR_SLOT_FREE_FUNCTION, OBJECT_READ_IS_COMPLETE_FUNCTION,
    OBJECT_READ_SYSTEM_OBJECT, RELEASE_OBJECT_READ_FUNCTION, REQUEST_OBJECT_READ_FUNCTION,
    SPAWN_BY_NAME_FUNCTION,
};
use automata_game::enemies::{
    COMPONENT_MAP_COUNT_OFFSET, COMPONENT_MAP_ENTRIES_POINTER_OFFSET, COMPONENT_MAP_ENTRY_LEN,
    ENTITY_COMPONENT_HOLDER_POINTER_OFFSET, EnemySpawn,
    GENERATOR_GROUP_WORD_OFFSET_IN_THE_COMPONENT, GENERATOR_HEADER_LEN, GENERATOR_SET_LEN,
    GENERATOR_SLOT_ARRAY_ADD_VTABLE_OFFSET, GENERATOR_SLOT_ARRAY_OFFSET, GENERATOR_SLOT_LEN,
    GeneratorGroupWord, GeneratorHeader, GeneratorSlot, MOST_COMPONENT_MAP_ENTRIES_READ,
    MOST_GENERATORS_WALKED, MOST_SETS_OF_ONE_GENERATOR, MOST_SLOTS_READ_OF_ONE_GENERATOR,
    SPAWN_BLOCK_LEN, component_in_map, generator_header, generator_slots, new_slot_bytes,
    object_id_of_set, spawn_block_bytes, spawn_values_of_set,
};
use automata_game::partner::{BODY_POSITION_OFFSET, ENTITY_HANDLE_OFFSET};

use crate::game_calls::GameCalls;

const NAME_OF_A_GENERATORS_ENEMY: &[u8] = b"\0";
const RELEASE_OBJECT_READ_UNUSED_ARGUMENT: u64 = 0;

type ObjectReadCall = unsafe extern "system" fn(usize, u32, u32) -> i32;
type ReleaseObjectRead = unsafe extern "system" fn(usize, u32, u32, u64);
type SpawnByName = unsafe extern "system" fn(usize, *const u8, u32, *const SpawnBlock) -> usize;
type SlotAdd = unsafe extern "system" fn(usize, *const NewSlot) -> u8;
type SlotFree = unsafe extern "system" fn(usize);

#[repr(C, align(16))]
struct NewSlot([u8; GENERATOR_SLOT_LEN]);

#[repr(C, align(16))]
struct SpawnBlock([u8; SPAWN_BLOCK_LEN]);

impl GameCalls {
    /// The word a generator writes into the enemies it made, read the way its write finds the
    /// place (entity +0x60, the component map at +0x6A8, the kind the game numbered). Reads only.
    pub fn generator_group_word_of_handle(&self, handle: u32) -> GeneratorGroupWord {
        let entity = self.entity_of_handle(handle);
        if entity == 0 {
            return GeneratorGroupWord::Unreadable;
        }
        let read = || -> Option<GeneratorGroupWord> {
            let holder = self
                .read_u64_at(entity as u64 + ENTITY_COMPONENT_HOLDER_POINTER_OFFSET)
                .ok()
                .filter(|holder| *holder != 0)?;
            let kind = self.read_i32(GENERATOR_GROUP_COMPONENT_KIND_VALUE).ok()?;
            let entries = self
                .read_u64_at(holder + COMPONENT_MAP_ENTRIES_POINTER_OFFSET)
                .ok()?;
            let count = self.read_u64_at(holder + COMPONENT_MAP_COUNT_OFFSET).ok()?;
            if count == 0 || entries == 0 {
                return Some(GeneratorGroupWord::NoComponent);
            }
            if count > MOST_COMPONENT_MAP_ENTRIES_READ {
                return None;
            }
            let bytes = self
                .read_bytes_at(entries, count as usize * COMPONENT_MAP_ENTRY_LEN)
                .ok()?;
            let Some(component) = component_in_map(&bytes, kind) else {
                return Some(GeneratorGroupWord::NoComponent);
            };
            let word = self
                .read_bytes_at(component + GENERATOR_GROUP_WORD_OFFSET_IN_THE_COMPONENT, 2)
                .ok()?;
            Some(GeneratorGroupWord::Word(u16::from_le_bytes([
                word[0], word[1],
            ])))
        };
        read().unwrap_or(GeneratorGroupWord::Unreadable)
    }

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

    /// The object read variant and the four set values the game's spawn would hand an enemy of this set.
    pub fn spawn_values_of_generator_set(
        &self,
        header: &GeneratorHeader,
        set_index: u32,
    ) -> Option<(u32, [i32; 4])> {
        if header.sets == 0 || u64::from(set_index) >= header.set_count.min(256) {
            return None;
        }
        let bytes = self
            .read_bytes_at(
                header.sets + u64::from(set_index) * GENERATOR_SET_LEN as u64,
                GENERATOR_SET_LEN,
            )
            .ok()?;
        spawn_values_of_set(&bytes, header.uses_the_second_set_values)
    }

    pub fn body_position(&self, body: usize) -> Option<[f32; 4]> {
        self.read_f32s_at(body as u64 + u64::from(BODY_POSITION_OFFSET))
            .ok()
    }

    /// Asks the game to read an enemy's data, as a generator's slot does before it spawns.
    #[expect(unsafe_code, reason = "calls the game's object read request")]
    pub fn request_enemy_read(&self, spawn: &EnemySpawn) -> bool {
        // SAFETY: the address is the object read system's request of the verified build, called
        // with its own global, the object id and the variant the host's generator used.
        unsafe {
            core::mem::transmute::<usize, ObjectReadCall>(
                self.address_of(REQUEST_OBJECT_READ_FUNCTION),
            )(
                self.address_of(OBJECT_READ_SYSTEM_OBJECT),
                spawn.object_id,
                spawn.object_read_variant,
            ) != 0
        }
    }

    #[expect(unsafe_code, reason = "calls the game's object read query")]
    pub fn enemy_read_is_complete(&self, spawn: &EnemySpawn) -> bool {
        // SAFETY: the address is a query of the object read system of the verified build; it
        // reads its tables under its own locks.
        unsafe {
            core::mem::transmute::<usize, ObjectReadCall>(
                self.address_of(OBJECT_READ_IS_COMPLETE_FUNCTION),
            )(
                self.address_of(OBJECT_READ_SYSTEM_OBJECT),
                spawn.object_id,
                spawn.object_read_variant,
            ) != 0
        }
    }

    #[expect(unsafe_code, reason = "calls the game's object read release")]
    pub fn release_enemy_read(&self, object_id: u32, object_read_variant: u32) {
        // SAFETY: the address is the object read system's release of the verified build, called
        // with its own global for a read this mod requested.
        unsafe {
            core::mem::transmute::<usize, ReleaseObjectRead>(
                self.address_of(RELEASE_OBJECT_READ_FUNCTION),
            )(
                self.address_of(OBJECT_READ_SYSTEM_OBJECT),
                object_id,
                object_read_variant,
                RELEASE_OBJECT_READ_UNUSED_ARGUMENT,
            );
        }
    }

    /// Makes the enemy with the spawn call and block a generator's slot uses; its handle, or none.
    /// # Safety
    /// The read of the enemy's data must be complete; game thread only.
    #[expect(
        unsafe_code,
        reason = "calls the game's spawn by name with a spawn block"
    )]
    pub unsafe fn spawn_enemy_without_a_generator(&self, spawn: &EnemySpawn) -> Option<u32> {
        let block = SpawnBlock(spawn_block_bytes(
            &spawn.position,
            &spawn.rotation,
            &spawn.set_values,
            spawn.level.unwrap_or(1),
        ));
        // SAFETY: the address is spawnByName of the verified build and the manager its global;
        // the empty name and the 0x90-byte block are what a generator's slot hands it, and the
        // block lives and is aligned for the call. The caller upholds the contract above.
        let entity = unsafe {
            core::mem::transmute::<usize, SpawnByName>(self.address_of(SPAWN_BY_NAME_FUNCTION))(
                self.address_of(ENTITY_MANAGER_OBJECT),
                NAME_OF_A_GENERATORS_ENEMY.as_ptr(),
                spawn.object_id,
                &block,
            )
        };
        if entity == 0 {
            return None;
        }
        self.read_u32_at(entity as u64 + u64::from(ENTITY_HANDLE_OFFSET))
            .ok()
            .filter(|handle| *handle != 0)
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
