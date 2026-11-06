use crate::zone_rules::ExitDoor;

pub static EXIT_DOORS: [ExitDoor; 30] = [
    ExitDoor {
        phase: "A_CENTER_LIBRARY_01",
        door_id: Some(601),
        via: "JUMP_MAP_01_01",
    },
    ExitDoor {
        phase: "A_CENTER_VILLAGE_01",
        door_id: Some(603),
        via: "JUMP_MAP_04_01",
    },
    ExitDoor {
        phase: "A_MOUNT_FIELD_01",
        door_id: Some(603),
        via: "JUMP_MAP_01_01",
    },
    ExitDoor {
        phase: "A_NORTH_FIELD_01",
        door_id: Some(601),
        via: "JUMP_MAP_03_01",
    },
    ExitDoor {
        phase: "A_SOUTH_MANSION_02",
        door_id: None,
        via: "JUMP_MAP_01",
    },
    ExitDoor {
        phase: "A_STONE_FIELD_01",
        door_id: Some(601),
        via: "JUMP_MAP_03_02",
    },
    ExitDoor {
        phase: "A_STONE_FIELD_01",
        door_id: Some(602),
        via: "JUMP_MAP_04_02",
    },
    ExitDoor {
        phase: "A_STONE_FIELD_02",
        door_id: Some(601),
        via: "JUMP_MAP_01_01",
    },
    ExitDoor {
        phase: "A_STONE_TEMPLE_01",
        door_id: Some(610),
        via: "JUMP_MAP_02",
    },
    ExitDoor {
        phase: "B_CASTLE_FIELD_01",
        door_id: Some(604),
        via: "JUMP_MAP_02",
    },
    ExitDoor {
        phase: "B_CASTLE_FIELD_01",
        door_id: Some(605),
        via: "JUMP_MAP_01",
    },
    ExitDoor {
        phase: "B_CASTLE_FIELD_02",
        door_id: Some(601),
        via: "JUMP_MAP_01_01",
    },
    ExitDoor {
        phase: "B_CASTLE_FIELD_02",
        door_id: Some(607),
        via: "JUMP_MAP_02_01",
    },
    ExitDoor {
        phase: "B_CENTER_LIBRARY_01",
        door_id: Some(601),
        via: "JUMP_MAP_01_01",
    },
    ExitDoor {
        phase: "B_CENTER_VILLAGE_01",
        door_id: Some(603),
        via: "JUMP_MAP_04_01",
    },
    ExitDoor {
        phase: "B_MOUNT_FIELD_01",
        door_id: Some(603),
        via: "JUMP_MAP_01_01",
    },
    ExitDoor {
        phase: "B_NORTH_FIELD_01",
        door_id: Some(601),
        via: "JUMP_MAP_03_01",
    },
    ExitDoor {
        phase: "B_SEASIDE_TOWN_01",
        door_id: Some(601),
        via: "EID_9320_a0040_0010_Start",
    },
    ExitDoor {
        phase: "B_SEASIDE_TOWN_01",
        door_id: Some(602),
        via: "EID_9320_a0120_0010_Start",
    },
    ExitDoor {
        phase: "B_STONE_FIELD_01",
        door_id: Some(601),
        via: "JUMP_MAP_03",
    },
    ExitDoor {
        phase: "B_STONE_FIELD_01",
        door_id: Some(602),
        via: "JUMP_MAP_04_01",
    },
    ExitDoor {
        phase: "B_STONE_FIELD_02",
        door_id: Some(601),
        via: "JUMP_MAP_01_01",
    },
    ExitDoor {
        phase: "B_STONE_TEMPLE_01",
        door_id: Some(610),
        via: "JUMP_MAP_02",
    },
    ExitDoor {
        phase: "E_NORTH_FIELD_01",
        door_id: Some(601),
        via: "JUMP_MAP_03_01",
    },
    ExitDoor {
        phase: "X_CENTER_LIBRARY_011",
        door_id: Some(601),
        via: "JUMP_MAP_01_01",
    },
    ExitDoor {
        phase: "X_DLC_START_MAP",
        door_id: Some(1),
        via: "JUMP_MAP_CENTER_VILLAGE",
    },
    ExitDoor {
        phase: "X_DLC_START_MAP",
        door_id: Some(2),
        via: "JUMP_MAP_DLC_01",
    },
    ExitDoor {
        phase: "X_DLC_START_MAP",
        door_id: Some(3),
        via: "JUMP_MAP_DLC_06",
    },
    ExitDoor {
        phase: "X_DLC_START_MAP",
        door_id: Some(4),
        via: "JUMP_MAP_DLC_11",
    },
    ExitDoor {
        phase: "X_STONE_FIELD_021",
        door_id: Some(601),
        via: "JUMP_MAP_01_01",
    },
];
