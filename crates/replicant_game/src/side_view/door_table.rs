/// A place whose side view only a door's script sets. `entry_steps` and `exit_steps` are the
/// callbacks the mod may run for a player it moved; empty means none may be run. The shell parts
/// and the outdoor camera status are what the exit restores, for a door with no exit steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideViewDoor {
    pub id: u16,
    pub area: &'static str,
    pub entry_callback: &'static str,
    pub entry_steps: &'static [&'static str],
    pub exit_steps: &'static [&'static str],
    pub indoor_start_points: &'static [i32],
    pub shell_terrain_parts: &'static [&'static str],
    pub outdoor_camera_status: i32,
    pub source: &'static str,
}

pub static SIDE_VIEW_DOORS: [SideViewDoor; 5] = [
    SideViewDoor {
        id: 1,
        area: "A_CENTER_VILLAGE_01",
        entry_callback: "GM_IN_Door_of_NierHouse_02",
        entry_steps: &["GM_IN_Door_of_NierHouse_02", "GM_IN_Door_of_NierHouse_03"],
        exit_steps: &[],
        indoor_start_points: &[8],
        shell_terrain_parts: &["PARTS_GROUP001"],
        outdoor_camera_status: 1,
        source: "listing c_A_CENTER_VILLAGE_01 main/279 to main/291; every exit step starts story events",
    },
    SideViewDoor {
        id: 2,
        area: "A_CENTER_LIBRARY_01",
        entry_callback: "GM_IN_Door_of_PopolRoom_02",
        entry_steps: &["GM_IN_Door_of_PopolRoom_02", "GM_IN_Door_of_PopolRoom_03"],
        exit_steps: &["GM_OUT_Door_of_PopolRoom_02"],
        indoor_start_points: &[],
        shell_terrain_parts: &["PARTS_GROUP004", "PARTS_GROUP002"],
        outdoor_camera_status: 1,
        source: "listing c_A_CENTER_LIBRARY_01 main/96 to main/103",
    },
    SideViewDoor {
        id: 3,
        area: "A_CENTER_LIBRARY_01",
        entry_callback: "GM_IN_Door_of_PopolRoom2_02",
        entry_steps: &["GM_IN_Door_of_PopolRoom2_02", "GM_IN_Door_of_PopolRoom2_03"],
        exit_steps: &["GM_OUT_Door_of_PopolRoom2_02"],
        indoor_start_points: &[],
        shell_terrain_parts: &["PARTS_GROUP004"],
        outdoor_camera_status: 1,
        source: "listing c_A_CENTER_LIBRARY_01 main/110 to main/114",
    },
    SideViewDoor {
        id: 4,
        area: "A_CENTER_LIBRARY_01",
        entry_callback: "GM_IN_Door_of_WestRoom_02",
        entry_steps: &[],
        exit_steps: &["GM_OUT_Door_of_WestRoom_02"],
        indoor_start_points: &[],
        shell_terrain_parts: &["PARTS_GROUP004", "PARTS_GROUP002"],
        outdoor_camera_status: 1,
        source: "listing c_A_CENTER_LIBRARY_01 main/104 to main/108; the entry's last step sets a flag and starts a talk",
    },
    SideViewDoor {
        id: 5,
        area: "A_CENTER_LIBRARY_01",
        entry_callback: "GM_IN_Door_of_WestRoom2_02",
        entry_steps: &[],
        exit_steps: &["GM_OUT_Door_of_WestRoom2_02"],
        indoor_start_points: &[],
        shell_terrain_parts: &["PARTS_GROUP004"],
        outdoor_camera_status: 3,
        source: "listing c_A_CENTER_LIBRARY_01 main/115 to main/119; the entry's last step sets a flag and starts a talk",
    },
];
