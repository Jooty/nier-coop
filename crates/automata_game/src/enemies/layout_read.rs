use crate::enemies::{
    EMPTY_ENTITY_HANDLES, ENTITY_LAYOUT_ENTITY_HANDLE_OFFSET, ENTITY_LAYOUT_ID_OFFSET,
    ENTITY_LAYOUT_LEN, ENTITY_LAYOUT_OBJECT_ID_OFFSET, ENTITY_LAYOUTS_HEADER_LEN,
    ENTITY_LAYOUTS_ID_OFFSET, ENTITY_LAYOUTS_ITEM_COUNT_OFFSET,
    ENTITY_LAYOUTS_ITEMS_POINTER_OFFSET, ENTITY_LAYOUTS_NEXT_POINTER_OFFSET,
    ENTITY_TABLE_SLOT_ENTITY_POINTER_OFFSET, ENTITY_TABLE_SLOT_LEN, EnemyId,
    HIGHEST_ENEMY_HEALTH_ON_THE_WIRE, HIGHEST_ENEMY_OBJECT_ID, LOWEST_ENEMY_OBJECT_ID,
};

/// The fields of one entity layouts list (`hap::EntityLayoutsMixin`) the census reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutsHeader {
    pub items: u64,
    pub count: u64,
    pub id: u32,
    pub next: u64,
}

/// One entry of an entity layouts list that holds an enemy the game has made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub id: EnemyId,
    pub entry_id: u32,
    pub object_id: u32,
    pub handle: u32,
}

pub fn layouts_header(bytes: &[u8]) -> Option<LayoutsHeader> {
    if bytes.len() < ENTITY_LAYOUTS_HEADER_LEN {
        return None;
    }
    Some(LayoutsHeader {
        items: u64_at(bytes, ENTITY_LAYOUTS_ITEMS_POINTER_OFFSET)?,
        count: u64_at(bytes, ENTITY_LAYOUTS_ITEM_COUNT_OFFSET)?,
        id: u32_at(bytes, ENTITY_LAYOUTS_ID_OFFSET)?,
        next: u64_at(bytes, ENTITY_LAYOUTS_NEXT_POINTER_OFFSET)?,
    })
}

/// The entries of one list's item bytes that place an enemy and hold a handle; a cut-off last entry is left out.
pub fn enemy_placements(layout_id: u32, items: &[u8]) -> Vec<Placement> {
    items
        .as_chunks::<ENTITY_LAYOUT_LEN>()
        .0
        .iter()
        .filter_map(|entry| {
            let object_id = u32_at(entry, ENTITY_LAYOUT_OBJECT_ID_OFFSET)?;
            let handle = u32_at(entry, ENTITY_LAYOUT_ENTITY_HANDLE_OFFSET)?;
            let entry_id = u32_at(entry, ENTITY_LAYOUT_ID_OFFSET)?;
            (is_enemy_object_id(object_id) && !EMPTY_ENTITY_HANDLES.contains(&handle)).then_some(
                Placement {
                    id: EnemyId::Placed {
                        layout: layout_id,
                        entry: entry_id,
                    },
                    entry_id,
                    object_id,
                    handle,
                },
            )
        })
        .collect()
}

/// The handle and the entity pointer of every used slot of the game's entity table.
pub fn live_entity_slots(table: &[u8]) -> Vec<(u32, u64)> {
    table
        .as_chunks::<ENTITY_TABLE_SLOT_LEN>()
        .0
        .iter()
        .filter_map(|slot| {
            let handle = u32_at(slot, 0)?;
            let entity = u64_at(slot, ENTITY_TABLE_SLOT_ENTITY_POINTER_OFFSET)?;
            (handle != 0 && entity != 0).then_some((handle, entity))
        })
        .collect()
}

pub fn is_enemy_object_id(object_id: u32) -> bool {
    (LOWEST_ENEMY_OBJECT_ID..=HIGHEST_ENEMY_OBJECT_ID).contains(&object_id)
}

/// Other classes share the enemy id range and keep other things where an enemy keeps its health:
/// an enemy's full health is a count in the range the lists carry, and its health is not above it.
pub fn reads_as_an_enemys_health(health: i32, max_health: i32) -> bool {
    (1..=HIGHEST_ENEMY_HEALTH_ON_THE_WIRE).contains(&max_health)
        && (-HIGHEST_ENEMY_HEALTH_ON_THE_WIRE..=max_health).contains(&health)
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let found = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([found[0], found[1], found[2], found[3]]))
}

fn u64_at(bytes: &[u8], offset: usize) -> Option<u64> {
    let low = u32_at(bytes, offset)?;
    let high = u32_at(bytes, offset + 4)?;
    Some(u64::from(high) << 32 | u64::from(low))
}
