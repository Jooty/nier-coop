use crate::address::GameAddress;

pub const ENTRY_POINT: GameAddress = GameAddress::new(0x9D_6444, 0x9C_DB64);

pub const DIRECT_INPUT8_CREATE_IMPORT_SLOT: GameAddress = GameAddress::new(0xC1_B000, 0xC1_1000);

pub const FRAME_END_FUNCTION: GameAddress = GameAddress::new(0x24_A250, 0x24_37D0);

pub const POLL_DEVICES_FUNCTION: GameAddress = GameAddress::new(0x27_7CD0, 0x27_24B0);

pub const POLL_MOUSE_FUNCTION: GameAddress = GameAddress::new(0x27_8FB0, 0x27_3790);

pub const CURSOR_RECENTRE_FUNCTION: GameAddress = GameAddress::new(0x27_84F0, 0x27_2CD0);

pub const PAD_INPUT_SOURCE: GameAddress = GameAddress::new(0x177_FD80, 0x177_3BA0);

pub const KEYBOARD_INPUT_SOURCE: GameAddress = GameAddress::new(0x177_FFB0, 0x177_3DD0);

pub const MOUSE_INPUT_SOURCE: GameAddress = GameAddress::new(0x179_D8C8, 0x179_16E8);

pub const CREATE_EVENT_IMPORT_SLOT: GameAddress = GameAddress::new(0xC1_B040, 0xC1_1040);

pub const BUILD_SAVE_FILE_PATH_FUNCTION: GameAddress = GameAddress::new(0x28_2CD0, 0x27_D250);

pub const SCENE_STATE_VALUE: GameAddress = GameAddress::new(0x102_A368, 0x101_E1E8);

pub const GAME_PAUSE_VALUE: GameAddress = GameAddress::new(0x11E_AC2C, 0x11E_0D5C);

pub const CURRENT_PHASE_ID_VALUE: GameAddress = GameAddress::new(0xF6_4AA0, 0xF5_8A90);

pub const CURRENT_SUB_PHASE_NAME_VALUE: GameAddress = GameAddress::new(0xF6_4AA8, 0xF5_8A98);

pub const PENDING_PHASE_ID_VALUE: GameAddress = GameAddress::new(0xF6_4B18, 0xF5_8B08);

pub const SCENE_LOADED_FLAG_VALUE: GameAddress = GameAddress::new(0x176_E010, 0x176_1E30);

pub const SAVE_MANAGER_IS_BUSY_FUNCTION: GameAddress = GameAddress::new(0x9C_62B0, 0x9B_DA00);

pub const CHAPTER_DATA_POINTER: GameAddress = GameAddress::new(0x11E_ABC8, 0x11E_0CF8);

pub const SCENE_STATE_SYSTEM_OBJECT: GameAddress = GameAddress::new(0xFC_2680, 0xFB_6580);

pub const NAME_HASH_FUNCTION: GameAddress = GameAddress::new(0x28_EE30, 0x28_9400);

pub const NAME_INTERN_FUNCTION: GameAddress = GameAddress::new(0x15_0E70, 0x15_0C30);

pub const NAME_INTERN_TABLE: GameAddress = GameAddress::new(0xF8_C4E0, 0xF8_03B0);

pub const SCENE_STATE_SET_FLAG_FUNCTION: GameAddress = GameAddress::new(0x35_4B20, 0x34_EF00);

pub const CHAPTER_MARKER_FUNCTION: GameAddress = GameAddress::new(0x9C_3B30, 0x9B_B280);

pub const CLEAR_FLAG_BIT_FUNCTION: GameAddress = GameAddress::new(0x9C_BD60, 0x9C_3490);

pub const STARTED_FROM_CHAPTER_SELECT_VALUE: GameAddress = GameAddress::new(0xF6_4040, 0xF5_8030);

pub const PHASE_START_FUNCTION: GameAddress = GameAddress::new(0x9C_6340, 0x9B_DA90);

pub const UI_BOOKKEEPING_WORD_VALUE: GameAddress = GameAddress::new(0x102_9B00, 0x101_D980);

pub const UI_BOOKKEEPING_FLAG_VALUE: GameAddress = GameAddress::new(0x100_6A04, 0xFF_A8C4);

pub const HERO_MODEL_ID_VALUE: GameAddress = GameAddress::new(0x11E_1F84, 0x11D_80AC);

pub const CURRENT_HERO_FUNCTION: GameAddress = GameAddress::new(0x7C_4F50, 0x7B_CC10);

pub const SCENE_OBJECT: GameAddress = GameAddress::new(0x102_A360, 0x101_E1E0);

pub const REQUEST_PHASE_CHANGE_FUNCTION: GameAddress = GameAddress::new(0x76_14A0, 0x75_9370);

pub const SET_PENDING_SUB_PHASE_FUNCTION: GameAddress = GameAddress::new(0x85_F290, 0x85_6B30);

pub const REQUEST_LOAD_SLOT_FUNCTION: GameAddress = GameAddress::new(0x9C_6C00, 0x9B_E350);

pub const SAVED_PHASE_ID_VALUE: GameAddress = GameAddress::new(0x149_58B4, 0x148_9734);

pub const SAVED_PHASE_NAME_VALUE: GameAddress = GameAddress::new(0x149_58B8, 0x148_9738);

pub const POSSESSED_HANDLE_VALUE: GameAddress = GameAddress::new(0x125_05DC, 0x124_445C);

pub const CURRENT_BUDDY_FUNCTION: GameAddress = GameAddress::new(0x7C_4610, 0x7B_C2D0);

pub const ENTITY_MANAGER_OBJECT: GameAddress = GameAddress::new(0x102_9898, 0x101_D718);

pub const SPAWN_BY_NAME_FUNCTION: GameAddress = GameAddress::new(0x74_5C80, 0x73_DCB0);

pub const ENTITY_FROM_HANDLE_FUNCTION: GameAddress = GameAddress::new(0x74_5F20, 0x73_DF50);

pub const ENTITY_AS_PLAYER_BODY_FUNCTION: GameAddress = GameAddress::new(0x30_85C0, 0x30_2B90);

pub const ENTITY_TERMINATE_FUNCTION: GameAddress = GameAddress::new(0x74_6690, 0x73_E6C0);

pub const SET_BUDDY_HANDLE_FUNCTION: GameAddress = GameAddress::new(0x4F_0300, 0x4E_9EF0);

pub const DESTROY_BUDDY_FUNCTION: GameAddress = GameAddress::new(0x48_6390, 0x48_0420);

pub const TELEPORT_RESET_HAP_FUNCTION: GameAddress = GameAddress::new(0x4F_0FA0, 0x4E_AB70);

pub const PLAYER_BODY_VTABLE: GameAddress = GameAddress::new(0xC6_1628, 0xC6_8948);

pub const PROCESS_BUTTONS_FUNCTION: GameAddress = GameAddress::new(0x52_A790, 0x52_4140);

pub const STICK_UPDATE_FUNCTION: GameAddress = GameAddress::new(0x52_AD90, 0x52_4740);

pub const CONTROLLER_UPDATE_INPUT_CALLS: GameAddress = GameAddress::new(0x50_1103, 0x4F_ABE3);

pub const SAVE_LOAD_FAILED_FUNCTION: GameAddress = GameAddress::new(0x9C_61F0, 0x9B_D940);

pub const APPLY_LOADED_STORY_FUNCTION: GameAddress = GameAddress::new(0x9C_6B60, 0x9B_E2B0);

pub const NAME_RELEASE_FUNCTION: GameAddress = GameAddress::new(0x15_0C60, 0x15_0A20);

pub const FAST_TRAVEL_OBJECT: GameAddress = GameAddress::new(0xFC_0130, 0xFB_4030);

pub const FAST_TRAVEL_TO_POINT_FUNCTION: GameAddress = GameAddress::new(0x35_8860, 0x35_2CD0);

pub const SET_BUDDY_AI_TYPE_FUNCTION: GameAddress = GameAddress::new(0x4E_9DD0, 0x4E_3BF0);

pub const PARTNER_WALK_IN_STATE_3B_FUNCTION: GameAddress = GameAddress::new(0x4E_A070, 0x4E_3E90);

pub const START_ANIMATION_FUNCTION: GameAddress = GameAddress::new(0x43_3C70, 0x44_64C0);

pub const ACTION_UPDATE_FUNCTION: GameAddress = GameAddress::new(0x42_C530, 0x43_EE70);

pub const BODY_UPDATE_FUNCTION: GameAddress = GameAddress::new(0x42_A190, 0x43_CAD0);

pub const BODY_UPDATE_INPUT_MODE_COPY: GameAddress = GameAddress::new(0x42_A1C5, 0x43_CB05);

pub const KEYBOARD_AND_MOUSE_INPUT_MODE_VALUE: GameAddress =
    GameAddress::new(0x13F_C404, 0x13F_0298);

pub const CAMERA_OBJECT: GameAddress = GameAddress::new(0x102_0BB0, 0x101_4A30);

pub const CAMERA_MODE_REQUEST_FLAG_WORD_VALUE: GameAddress =
    GameAddress::new(0x102_9B04, 0x101_D984);

pub const GAME_FOCUS_LOST_TEST_FUNCTION: GameAddress = GameAddress::new(0x9C_6250, 0x9B_D9A0);

pub const SCENE_INPUT_OFF_FLAG_WORD_VALUE: GameAddress = GameAddress::new(0x102_9B10, 0x101_D990);

pub const EVENT_INPUT_OFF_FLAG_WORD_VALUE: GameAddress = GameAddress::new(0x102_9B14, 0x101_D994);

pub const CONTROLLER_STOPPING_MODE_VALUE: GameAddress = GameAddress::new(0x141_47B8, 0x140_8628);

pub const TRAVEL_POINT_IS_SKIPPED_BY_THE_FINDER_FUNCTION: GameAddress =
    GameAddress::new(0x3D_F2A0, 0x3D_96E0);

pub const OBJECT_READ_SYSTEM_OBJECT: GameAddress = GameAddress::new(0x13E_CC38, 0x13E_0AB8);

pub const REQUEST_OBJECT_READ_FUNCTION: GameAddress = GameAddress::new(0x85_11B0, 0x84_8A50);

pub const OBJECT_READ_IS_COMPLETE_FUNCTION: GameAddress = GameAddress::new(0x84_E950, 0x84_6220);

pub const RELEASE_OBJECT_READ_FUNCTION: GameAddress = GameAddress::new(0x85_0FB0, 0x84_8850);

pub const LOADED_OBJECT_DATA_FUNCTION: GameAddress = GameAddress::new(0x84_E130, 0x84_5A00);

pub const SCENE_TASK_SCHEDULER_OBJECT: GameAddress = GameAddress::new(0x102_9D58, 0x101_DCF8);

pub const SLEEP_CURRENT_TASK_FUNCTION: GameAddress = GameAddress::new(0x27_0260, 0x26_AA40);

pub const WAIT_FOR_OBJECT_READ_FUNCTION: GameAddress = GameAddress::new(0x75_EDE0, 0x75_6CB0);

pub const OBJECT_DATA_OR_WAIT_FUNCTION: GameAddress = GameAddress::new(0x84_E1F0, 0x84_5AC0);

pub const COMPAT_SAVE_LOAD_FAILED_FUNCTION_BYTES: [u8; 12] = [
    0x33, 0xC0, 0x39, 0x05, 0x98, 0x87, 0xA5, 0x00, 0x0F, 0x9C, 0xC0, 0xC3,
];

pub const COMPAT_APPLY_LOADED_STORY_FUNCTION_BYTES: [u8; 35] = [
    0x83, 0x3D, 0x29, 0x7E, 0xA5, 0x00, 0x00, 0x74, 0x03, 0x32, 0xC0, 0xC3, 0xC7, 0x05, 0x1A, 0x7E,
    0xA5, 0x00, 0x04, 0x00, 0x00, 0x00, 0xB0, 0x01, 0xC7, 0x05, 0x12, 0x7E, 0xA5, 0x00, 0x02, 0x00,
    0x00, 0x00, 0xC3,
];

pub const SCENE_LOADER_FUNCTION: GameAddress = GameAddress::new(0x76_23D0, 0x75_A2A0);

pub const SCENE_LOADER_STAGE_VALUE: GameAddress = GameAddress::new(0x102_9C2C, 0x101_DAB8);

pub const LOAD_DONE_REQUEST_VALUE: GameAddress = GameAddress::new(0x140_0954, 0x13F_47D4);

pub const LOADING_SCREEN_ANSWER_VALUE: GameAddress = GameAddress::new(0x140_0958, 0x13F_47D8);

pub const LOADING_SCREEN_OBJECT: GameAddress = GameAddress::new(0x140_07C0, 0x13F_4640);

pub const LOADING_SCREEN_STILL_LOADING_TEST_FUNCTION: GameAddress =
    GameAddress::new(0x8B_56E0, 0x8A_CEE0);

pub const ENGINE_PARTNER_SPAWN_FUNCTION: GameAddress = GameAddress::new(0x51_E7C0, 0x51_81A0);

pub const SCENE_STATE_FLAG_TEST_FUNCTION: GameAddress = GameAddress::new(0x49_5A70, 0x48_FB00);

pub const SCENE_STATE_CLEAR_FLAG_FUNCTION: GameAddress = GameAddress::new(0x34_F8E0, 0x34_9CC0);

pub const POD_SHOT_UPDATE_FUNCTION: GameAddress = GameAddress::new(0x48_A9D0, 0x48_4A60);

pub const CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION: GameAddress =
    GameAddress::new(0x18_4240, 0x18_3A80);

pub const DEFEAT_FLOW_STATE_VALUE: GameAddress = GameAddress::new(0x11E_AC1C, 0x11E_0D4C);

pub const DEFEAT_FLOW_STEP_VALUE: GameAddress = GameAddress::new(0x11E_AC20, 0x11E_0D50);

pub const RESTORE_POINT_POINTER_VALUE: GameAddress = GameAddress::new(0x142_2278, 0x141_60F8);

pub const PLAYER_DAMAGE_FUNCTION: GameAddress = GameAddress::new(0x43_4530, 0x44_6DF0);

pub const DEFEAT_LEFT_BODY_RECORD_FLAG_VALUE: GameAddress =
    GameAddress::new(0x11E_AC24, 0x11E_0D54);

pub const HUD_LOG_POST_TEXT_ID_FUNCTION: GameAddress = GameAddress::new(0x8C_A480, 0x8C_1C80);

pub const UI_TEXT_LOOKUP_FUNCTION: GameAddress = GameAddress::new(0x9B_D770, 0x9B_4EC0);

pub const ENTITY_LAYOUTS_LIST_HEAD_VALUE: GameAddress = GameAddress::new(0xFC_9880, 0xFB_D848);

pub const ENEMY_DAMAGE_FUNCTION: GameAddress = GameAddress::new(0x66_D2F0, 0x66_5F60);

pub const ENEMY_FLAT_DAMAGE_FUNCTION: GameAddress = GameAddress::new(0x66_D760, 0x66_63D0);

pub const ENEMY_FORCED_DEFEAT_STATE_TEST_FUNCTION: GameAddress =
    GameAddress::new(0x59_05D0, 0x58_9B60);

pub const ENTITY_TABLE_POINTER_VALUE: GameAddress = GameAddress::new(0x102_9860, 0x101_D6E0);

pub const ENTITY_TABLE_SLOT_COUNT_VALUE: GameAddress = GameAddress::new(0x102_9850, 0x101_D6D0);
