use rc_game::story_calls::{
    LIBRARY_PACK_HASH, STORY_CALL_TABLE, SceneScope, StoryCallKind, is_event_callback_name,
    kind_in_table, name_hash, story_call_kind, story_call_kind_by_name,
};

const PACK_A: u64 = 10;
const PACK_B: u64 = 20;
const FUNCTION_ONE: u64 = 1;
const FUNCTION_TWO: u64 = 2;
const FUNCTION_THREE: u64 = 3;

const SMALL_TABLE: [(u64, u64, StoryCallKind); 4] = [
    (PACK_A, FUNCTION_ONE, StoryCallKind::Story),
    (PACK_A, FUNCTION_TWO, StoryCallKind::Container),
    (PACK_B, FUNCTION_ONE, StoryCallKind::Container),
    (PACK_B, FUNCTION_TWO, StoryCallKind::Story),
];

#[test]
fn the_name_hash_is_fnv_1a_64() {
    assert_eq!(name_hash(""), 0xCBF2_9CE4_8422_2325);
    assert_eq!(name_hash("a"), 0xAF63_DC4C_8601_EC8C);
}

#[test]
fn the_library_hash_is_the_hash_of_its_pack_name() {
    assert_eq!(LIBRARY_PACK_HASH, name_hash("__lib__"));
}

#[test]
fn a_lookup_needs_both_the_pack_and_the_function() {
    assert_eq!(
        kind_in_table(&SMALL_TABLE, PACK_A, FUNCTION_ONE),
        Some(StoryCallKind::Story)
    );
    assert_eq!(
        kind_in_table(&SMALL_TABLE, PACK_B, FUNCTION_ONE),
        Some(StoryCallKind::Container)
    );
    assert_eq!(kind_in_table(&SMALL_TABLE, PACK_A, FUNCTION_THREE), None);
    assert_eq!(kind_in_table(&SMALL_TABLE, 30, FUNCTION_ONE), None);
}

#[test]
fn only_story_calls_are_relayed() {
    assert!(StoryCallKind::Story.is_relayed());
    assert!(!StoryCallKind::Container.is_relayed());
    assert!(!StoryCallKind::Local.is_relayed());
}

#[test]
fn the_table_is_sorted_with_unique_keys() {
    let keys: Vec<(u64, u64)> = STORY_CALL_TABLE.iter().map(|&(p, f, _)| (p, f)).collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn the_table_holds_no_local_entries() {
    assert!(
        STORY_CALL_TABLE
            .iter()
            .all(|&(_, _, kind)| kind != StoryCallKind::Local)
    );
}

#[test]
fn every_table_entry_is_found_by_the_real_lookup() {
    for &(pack, function, kind) in &STORY_CALL_TABLE {
        assert_eq!(story_call_kind(pack, function), kind);
    }
}

#[test]
fn a_function_in_no_table_row_is_local() {
    assert_eq!(story_call_kind(1, 1), StoryCallKind::Local);
    assert_eq!(
        story_call_kind_by_name("unknown_pack", "unknown_function"),
        StoryCallKind::Local
    );
}

#[test]
fn a_library_row_answers_for_any_pack() {
    let &(_, function, kind) = STORY_CALL_TABLE
        .iter()
        .find(|&&(pack, _, _)| pack == LIBRARY_PACK_HASH)
        .expect("the table has a library row");
    assert_eq!(story_call_kind(0x1234, function), kind);
}

const FIELD_PACK_HASH: u64 = 0x180F_D61C_3A1D_0F5C;
const FIELD_EVENT_END_HASH: u64 = 0x4C74_454B_E885_3F6B;
const FIELD_EVENT_START_HASH: u64 = 0x21A3_2335_129E_2C7E;

#[test]
fn the_event_systems_own_callbacks_are_local_in_the_real_table() {
    assert_eq!(
        story_call_kind(FIELD_PACK_HASH, FIELD_EVENT_END_HASH),
        StoryCallKind::Local
    );
    assert_eq!(
        story_call_kind(FIELD_PACK_HASH, FIELD_EVENT_START_HASH),
        StoryCallKind::Local
    );
}

#[test]
fn event_callback_names_are_recognised_by_their_prefix_and_suffix() {
    assert!(is_event_callback_name("EID_0050_a0020_End"));
    assert!(is_event_callback_name("EID_0050_a0020_Start"));
    assert!(is_event_callback_name("SEID_1350_S0010_a0010_End"));
    assert!(is_event_callback_name("RA_EID_9300_a0020g_0030_Start"));
    assert!(is_event_callback_name("snow_EID_9300_a0030g_0010_Start"));
    assert!(is_event_callback_name("EID_1320_a0020_01_End02"));
    assert!(!is_event_callback_name("EID_1350_a0038g_020_Talk"));
    assert!(!is_event_callback_name("Pick_Motion_End"));
    assert!(!is_event_callback_name("EID_Started"));
    assert!(!is_event_callback_name(""));
}

const NAMES_SEEN_IN_THE_FOURTEENTH_TEST: [(&str, SceneScope); 40] = [
    ("EID_0050_a0044g_Start", SceneScope::SharedScene),
    ("EID_0050_a0044g_End", SceneScope::SharedScene),
    ("EID_0100_a0012_Start", SceneScope::SharedScene),
    ("EID_0100_a0010_End", SceneScope::SharedScene),
    ("Boss_Battle_Start", SceneScope::SharedScene),
    ("Put_Enemy_AL1_160_00_Enemy33", SceneScope::SharedProgress),
    ("Put_Enemy_AL1_070", SceneScope::SharedProgress),
    ("ScriptMode_to_Enemy_02", SceneScope::SharedProgress),
    ("Collision_Put_02", SceneScope::SharedProgress),
    ("Destination_Remove_01", SceneScope::SharedProgress),
    ("EID_0100_a0012_001", SceneScope::SharedProgress),
    ("Disp_Tutorial_03_02", SceneScope::SharedProgress),
    ("BGM_EID_0050_a0010_DemoSkip", SceneScope::SharedProgress),
    ("Impact_Effect_01", SceneScope::SharedProgress),
    ("GM_OUT_Door_of_NierHouse_14", SceneScope::LocalScene),
    ("GM_OUT_Door_of_NierHouse_02", SceneScope::LocalScene),
    ("PlayerActionDoor", SceneScope::LocalScene),
    ("PlayerCloseDoor", SceneScope::LocalScene),
    ("CB_FreeCamera_Outdoor", SceneScope::LocalScene),
    ("CB_FreeCamera_Indoor", SceneScope::LocalScene),
    ("CB_FreeCamera_Indoor_BossBattleEnd", SceneScope::LocalScene),
    ("CB_Change_Bright", SceneScope::LocalScene),
    ("CB_SideView_SetBright_NierHome_Out", SceneScope::LocalScene),
    ("LOOK_AT_SAVEPOINT_CAMERA", SceneScope::LocalScene),
    ("Reset_Camera_Flag", SceneScope::LocalScene),
    ("Boss_Battle_Camera", SceneScope::LocalScene),
    ("Gaze_to_Yonah_01", SceneScope::LocalScene),
    ("From_2F_to_1F_14", SceneScope::LocalScene),
    ("Change_Bright_04", SceneScope::LocalScene),
    ("WaterHeight_Default", SceneScope::LocalScene),
    ("WaterHeight_Home", SceneScope::LocalScene),
    ("Stop_Env_23", SceneScope::LocalScene),
    ("Play_Env_06", SceneScope::LocalScene),
    ("PLAY_SE_ENV05", SceneScope::LocalScene),
    ("LIB_SNOW_ENV_CB_ADD", SceneScope::LocalScene),
    ("LIB_SNOW_RESET_ENV_CB", SceneScope::LocalScene),
    ("Talk_Text_01", SceneScope::LocalScene),
    ("A0050_a0042g_CYCLOPS_PadShake", SceneScope::LocalScene),
    ("SelectWarpPoint", SceneScope::SharedProgress),
    ("Put_Enemy_AL1_020_03", SceneScope::SharedProgress),
];

#[test]
fn every_name_from_the_fourteenth_test_has_its_scope() {
    for (name, scope) in NAMES_SEEN_IN_THE_FOURTEENTH_TEST {
        assert_eq!(SceneScope::of(name), scope, "{name}");
    }
}

#[test]
fn only_local_scene_calls_count_as_local() {
    assert!(SceneScope::LocalScene.is_local());
    assert!(!SceneScope::SharedScene.is_local());
    assert!(!SceneScope::SharedProgress.is_local());
}

#[test]
fn door_scene_chains_stay_on_the_game_that_pressed_the_door() {
    for name in [
        "door_201_open_01",
        "door_308_open_11",
        "knock_door_201",
        "DoorOpenCameraCollisionOff",
    ] {
        assert_eq!(SceneScope::of(name), SceneScope::LocalScene, "{name}");
    }
}

#[test]
fn door_named_story_progress_stays_shared() {
    for name in [
        "Get_DoorKey_B3F",
        "Law_Door03_Set",
        "CaptainRoomDoorLockSet",
        "Opne_HawkDoor2",
    ] {
        assert_eq!(SceneScope::of(name), SceneScope::SharedProgress, "{name}");
    }
}
