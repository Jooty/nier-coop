use automata_game::address::{
    ENEMY_DAMAGE_FUNCTION, ENEMY_FLAT_DAMAGE_FUNCTION, ENEMY_FORCED_DEFEAT_STATE_TEST_FUNCTION,
    ENTITY_LAYOUTS_LIST_HEAD_VALUE,
};
use automata_game::enemies::{
    ENEMY_DAMAGE_VTABLE_OFFSET, ENEMY_FLAGS_THE_FORCED_DEFEAT_SETS_OFFSETS,
    ENEMY_FLAT_DAMAGE_VTABLE_OFFSET, ENEMY_FORCED_DEFEAT_STATE,
    ENEMY_FORCED_DEFEAT_STATE_WHEN_THE_GAMES_TEST_SAYS_SO, ENEMY_MAX_HEALTH_OFFSET,
    ENEMY_PLACEMENT_ID_OFFSET, ENTITY_BEHAVIOR_POINTER_OFFSET, ENTITY_LAYOUT_LEN,
    ENTITY_LAYOUTS_HEADER_LEN, EnemySighting, MOST_ENTITY_LAYOUTS_LISTS_WALKED,
    MOST_ENTITY_LAYOUTS_READ_OF_ONE_LIST, Placement, enemy_placements, is_enemy_object_id,
    layouts_header,
};
use automata_game::partner::{
    BODY_BASE_HEALTH_OFFSET, BODY_ENTITY_POINTER_OFFSET, BODY_MODEL_ID_OFFSET,
    ENTITY_HANDLE_OFFSET, STATE_START_VTABLE_OFFSET,
};

use crate::game_calls::GameCalls;

type EnemyFlatDamage = unsafe extern "system" fn(usize, u32) -> u64;
type EnemyStateTest = unsafe extern "system" fn(usize) -> i32;
type EnemyStateStart = unsafe extern "system" fn(usize, i32, u32, u32, u32);

/// One placed enemy read from the game: what is told to the other copy, and where it lives here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyReading {
    pub sighting: EnemySighting,
    pub body: usize,
    pub handle: u32,
    pub uses_the_base_damage_code: bool,
    pub body_carries_the_entry_id: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlacedEnemies {
    pub lists: usize,
    pub readings: Vec<EnemyReading>,
}

impl GameCalls {
    /// Walks the game's entity layouts lists and reads every enemy an entry holds. Reads only.
    pub fn read_placed_enemies(&self) -> PlacedEnemies {
        let mut placed = PlacedEnemies::default();
        let mut list = self.read_u64(ENTITY_LAYOUTS_LIST_HEAD_VALUE).unwrap_or(0);
        while list != 0 && placed.lists < MOST_ENTITY_LAYOUTS_LISTS_WALKED {
            let Some(header) = self
                .read_bytes_at(list, ENTITY_LAYOUTS_HEADER_LEN)
                .ok()
                .and_then(|bytes| layouts_header(&bytes))
            else {
                break;
            };
            placed.lists += 1;
            let holds_items = header.items != 0
                && (1..=MOST_ENTITY_LAYOUTS_READ_OF_ONE_LIST).contains(&header.count);
            if holds_items
                && let Ok(items) =
                    self.read_bytes_at(header.items, header.count as usize * ENTITY_LAYOUT_LEN)
            {
                placed.readings.extend(
                    enemy_placements(header.id, &items)
                        .into_iter()
                        .filter_map(|placement| self.read_placed_enemy(placement)),
                );
            }
            list = header.next;
        }
        placed
    }

    pub fn read_enemy_health(&self, body: usize) -> Option<i32> {
        self.read_i32_at(body as u64 + u64::from(BODY_BASE_HEALTH_OFFSET))
            .ok()
    }

    /// The handle of the entity a body belongs to, read through the body's own entity pointer.
    pub fn entity_handle_of_body(&self, body: usize) -> Option<u32> {
        let entity = self
            .read_u64_at(body as u64 + u64::from(BODY_ENTITY_POINTER_OFFSET))
            .ok()
            .filter(|entity| *entity != 0)?;
        self.read_u32_at(entity + u64::from(ENTITY_HANDLE_OFFSET))
            .ok()
    }

    pub fn write_enemy_health(&self, body: usize, health: i32) -> bool {
        self.write_i32_at(body as u64 + u64::from(BODY_BASE_HEALTH_OFFSET), health)
            .is_ok()
    }

    /// Takes health with the game's own flat damage code, which keeps the enemy's health floors.
    /// # Safety
    /// `body` must be a live enemy whose vtable holds the base damage functions; game thread only.
    #[expect(
        unsafe_code,
        reason = "calls the game's flat damage function of an enemy"
    )]
    pub unsafe fn apply_flat_damage(&self, body: usize, amount: u32) {
        // SAFETY: the address is the base flat damage function of the verified build, slot 153
        // of the enemy's own vtable by the caller's contract; it takes the enemy and an amount.
        unsafe {
            core::mem::transmute::<usize, EnemyFlatDamage>(
                self.address_of(ENEMY_FLAT_DAMAGE_FUNCTION),
            )(body, amount);
        }
    }

    /// What the game's hit loop does for a hit that defeats outright; false when a write failed.
    /// # Safety
    /// `body` must be a live enemy whose vtable holds the base damage functions; game thread only.
    #[expect(
        unsafe_code,
        reason = "calls the game's defeat state test and the enemy's state start through its vtable"
    )]
    pub unsafe fn force_enemy_defeat(&self, body: usize) -> bool {
        let at = body as u64;
        let Ok(start_state) = self
            .read_u64_at(at)
            .and_then(|vtable| self.read_u64_at(vtable + u64::from(STATE_START_VTABLE_OFFSET)))
        else {
            return false;
        };
        let written = self.write_enemy_health(body, 0)
            && ENEMY_FLAGS_THE_FORCED_DEFEAT_SETS_OFFSETS
                .iter()
                .all(|offset| self.write_i32_at(at + u64::from(*offset), 1).is_ok());
        if !written || start_state == 0 {
            return false;
        }
        // SAFETY: the test is the function the hit loop calls with the enemy before it starts the
        // defeat state, and the state start is slot 18 of the enemy's own vtable, called with the
        // arguments the hit loop gives it; the caller upholds the contract above.
        unsafe {
            let other_state = core::mem::transmute::<usize, EnemyStateTest>(
                self.address_of(ENEMY_FORCED_DEFEAT_STATE_TEST_FUNCTION),
            )(body)
                != 0;
            let state = if other_state {
                ENEMY_FORCED_DEFEAT_STATE_WHEN_THE_GAMES_TEST_SAYS_SO
            } else {
                ENEMY_FORCED_DEFEAT_STATE
            };
            core::mem::transmute::<usize, EnemyStateStart>(start_state as usize)(
                body, state, 0, 0, 0,
            );
        }
        true
    }

    pub fn body_of_handle(&self, handle: u32) -> Option<usize> {
        let entity = self.entity_of_handle(handle);
        if entity == 0 {
            return None;
        }
        self.read_u64_at(entity as u64 + u64::from(ENTITY_BEHAVIOR_POINTER_OFFSET))
            .ok()
            .filter(|body| *body != 0)
            .map(|body| body as usize)
    }

    pub fn object_id_of_handle(&self, handle: u32) -> Option<u32> {
        let body = self.body_of_handle(handle)?;
        self.read_u32_at(body as u64 + u64::from(BODY_MODEL_ID_OFFSET))
            .ok()
    }

    fn read_placed_enemy(&self, placement: Placement) -> Option<EnemyReading> {
        let body = self.body_of_handle(placement.handle)?;
        let at = body as u64;
        let object_id = self
            .read_u32_at(at + u64::from(BODY_MODEL_ID_OFFSET))
            .ok()
            .filter(|object_id| is_enemy_object_id(*object_id))?;
        let health = self.read_enemy_health(body)?;
        let max_health = self
            .read_i32_at(at + u64::from(ENEMY_MAX_HEALTH_OFFSET))
            .ok()?;
        let id_on_the_body = self
            .read_u32_at(at + u64::from(ENEMY_PLACEMENT_ID_OFFSET))
            .ok()?;
        Some(EnemyReading {
            sighting: EnemySighting {
                id: placement.id,
                object_id,
                health,
                max_health,
            },
            body,
            handle: placement.handle,
            uses_the_base_damage_code: self.vtable_slot_is(
                body,
                ENEMY_DAMAGE_VTABLE_OFFSET,
                self.address_value(ENEMY_DAMAGE_FUNCTION),
            ) && self.vtable_slot_is(
                body,
                ENEMY_FLAT_DAMAGE_VTABLE_OFFSET,
                self.address_value(ENEMY_FLAT_DAMAGE_FUNCTION),
            ),
            body_carries_the_entry_id: id_on_the_body == placement.id.entry,
        })
    }

    fn vtable_slot_is(&self, body: usize, slot_offset: u32, function: u64) -> bool {
        self.read_u64_at(body as u64)
            .and_then(|vtable| self.read_u64_at(vtable + u64::from(slot_offset)))
            .is_ok_and(|found| found == function)
    }
}
