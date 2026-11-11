use automata_game::address::{
    ENEMY_DAMAGE_FUNCTION, ENEMY_FLAT_DAMAGE_FUNCTION, ENTITY_LAYOUTS_LIST_HEAD_VALUE,
};
use automata_game::enemies::{
    ENEMY_DAMAGE_VTABLE_OFFSET, ENEMY_FLAT_DAMAGE_VTABLE_OFFSET, ENEMY_MAX_HEALTH_OFFSET,
    ENEMY_PLACEMENT_ID_OFFSET, ENTITY_BEHAVIOR_POINTER_OFFSET, ENTITY_LAYOUT_LEN,
    ENTITY_LAYOUTS_HEADER_LEN, EnemySighting, MOST_ENTITY_LAYOUTS_LISTS_WALKED,
    MOST_ENTITY_LAYOUTS_READ_OF_ONE_LIST, Placement, enemy_placements, is_enemy_object_id,
    layouts_header,
};
use automata_game::partner::{BODY_BASE_HEALTH_OFFSET, BODY_MODEL_ID_OFFSET};

use crate::game_calls::GameCalls;

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
