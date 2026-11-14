use replicant_proto::WorldObjectKey;

use crate::blocks::cell_of;
use crate::world_objects::kind_of_vtable_rva;

pub fn world_object_key(kind_vtable_rva: u32, group: u16, position: [f32; 3]) -> WorldObjectKey {
    WorldObjectKey {
        kind_vtable_rva,
        group,
        start_cell: cell_of(position),
    }
}

pub fn key_text(key: &WorldObjectKey) -> String {
    let kind = kind_of_vtable_rva(key.kind_vtable_rva).map_or_else(
        || format!("{:#x}", key.kind_vtable_rva),
        |kind| kind.name.to_owned(),
    );
    let [x, y, z] = key.start_cell;
    format!("{kind} group {} from ({x} {y} {z})", key.group)
}
