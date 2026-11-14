#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectSync {
    Push,
    PushAndBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldObjectKind {
    pub name: &'static str,
    pub vtable_rva: u32,
    pub class_id: i32,
    pub sync: ObjectSync,
}

pub static WORLD_OBJECT_KINDS: [WorldObjectKind; 5] = [
    WorldObjectKind {
        name: "CActorBox",
        vtable_rva: 0x00B0_0150,
        class_id: 0xAB,
        sync: ObjectSync::Push,
    },
    WorldObjectKind {
        name: "CActorBox2",
        vtable_rva: 0x00B0_0AA0,
        class_id: 0xAC,
        sync: ObjectSync::Push,
    },
    WorldObjectKind {
        name: "CActorJumpBox",
        vtable_rva: 0x00B0_1D30,
        class_id: 0xAF,
        sync: ObjectSync::Push,
    },
    WorldObjectKind {
        name: "CActorJumpBox2",
        vtable_rva: 0x00B0_2670,
        class_id: 0xB0,
        sync: ObjectSync::Push,
    },
    WorldObjectKind {
        name: "CActorAttackBox",
        vtable_rva: 0x00B0_54B0,
        class_id: 0xB3,
        sync: ObjectSync::PushAndBreak,
    },
];

pub fn kind_of_vtable_rva(vtable_rva: u32) -> Option<&'static WorldObjectKind> {
    WORLD_OBJECT_KINDS
        .iter()
        .find(|kind| kind.vtable_rva == vtable_rva)
}
