use crate::enemies::{
    EMPTY_ENTITY_HANDLES, ENTITY_LAYOUT_ENTITY_HANDLE_OFFSET, ENTITY_LAYOUT_ID_OFFSET,
    ENTITY_LAYOUT_LEN, ENTITY_LAYOUT_OBJECT_ID_OFFSET, ENTITY_LAYOUTS_HEADER_LEN,
    ENTITY_LAYOUTS_ID_OFFSET, ENTITY_LAYOUTS_ITEM_COUNT_OFFSET,
    ENTITY_LAYOUTS_ITEMS_POINTER_OFFSET, ENTITY_LAYOUTS_NEXT_POINTER_OFFSET, EnemyId,
    HIGHEST_ENEMY_OBJECT_ID, LOWEST_ENEMY_OBJECT_ID,
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
                    id: EnemyId {
                        layout: layout_id,
                        entry: entry_id,
                    },
                    object_id,
                    handle,
                },
            )
        })
        .collect()
}

pub fn is_enemy_object_id(object_id: u32) -> bool {
    (LOWEST_ENEMY_OBJECT_ID..=HIGHEST_ENEMY_OBJECT_ID).contains(&object_id)
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
