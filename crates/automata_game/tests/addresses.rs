use automata_game::address::{DIRECT_INPUT8_CREATE_IMPORT_SLOT, ENTRY_POINT, GameAddress};
use automata_game::exe::ExeVariant;

#[test]
fn an_address_is_picked_by_exe_variant() {
    let address = GameAddress::new(0x100, 0x200);
    assert_eq!(address.rva(ExeVariant::Main), 0x100);
    assert_eq!(address.rva(ExeVariant::Compat), 0x200);
}

#[test]
fn the_entry_point_differs_between_the_exes() {
    assert_eq!(ENTRY_POINT.rva(ExeVariant::Main), 0x9D6444);
    assert_eq!(ENTRY_POINT.rva(ExeVariant::Compat), 0x9CDB64);
}

#[test]
fn the_direct_input_import_slot_is_picked_per_variant() {
    assert_eq!(
        DIRECT_INPUT8_CREATE_IMPORT_SLOT.rva(ExeVariant::Main),
        0xC1B000
    );
    assert_eq!(
        DIRECT_INPUT8_CREATE_IMPORT_SLOT.rva(ExeVariant::Compat),
        0xC11000
    );
}

#[test]
fn every_variant_has_a_distinct_label() {
    assert_eq!(ExeVariant::Main.label(), "main");
    assert_eq!(ExeVariant::Compat.label(), "compat");
}

#[test]
fn the_frame_end_function_differs_per_exe() {
    use automata_game::address::FRAME_END_FUNCTION;
    assert_eq!(FRAME_END_FUNCTION.rva(ExeVariant::Main), 0x24A250);
    assert_eq!(FRAME_END_FUNCTION.rva(ExeVariant::Compat), 0x2437D0);
}

#[test]
fn the_second_copy_and_save_addresses_are_picked_per_variant() {
    use automata_game::address::{BUILD_SAVE_FILE_PATH_FUNCTION, CREATE_EVENT_IMPORT_SLOT};
    assert_eq!(CREATE_EVENT_IMPORT_SLOT.rva(ExeVariant::Main), 0xC1B040);
    assert_eq!(CREATE_EVENT_IMPORT_SLOT.rva(ExeVariant::Compat), 0xC11040);
    assert_eq!(
        BUILD_SAVE_FILE_PATH_FUNCTION.rva(ExeVariant::Main),
        0x282CD0
    );
    assert_eq!(
        BUILD_SAVE_FILE_PATH_FUNCTION.rva(ExeVariant::Compat),
        0x27D250
    );
}

#[test]
fn the_partner_body_addresses_are_the_ones_of_the_pick_page() {
    use automata_game::address::{
        CURRENT_BUDDY_FUNCTION, DESTROY_BUDDY_FUNCTION, ENTITY_AS_PLAYER_BODY_FUNCTION,
        ENTITY_FROM_HANDLE_FUNCTION, ENTITY_MANAGER_OBJECT, ENTITY_TERMINATE_FUNCTION,
        PLAYER_BODY_VTABLE, POSSESSED_HANDLE_VALUE, SET_BUDDY_HANDLE_FUNCTION,
        SPAWN_BY_NAME_FUNCTION, TELEPORT_RESET_HAP_FUNCTION,
    };
    let expected = [
        (SPAWN_BY_NAME_FUNCTION, 0x745C80, 0x73DCB0),
        (ENTITY_MANAGER_OBJECT, 0x1029898, 0x101D718),
        (ENTITY_FROM_HANDLE_FUNCTION, 0x745F20, 0x73DF50),
        (ENTITY_AS_PLAYER_BODY_FUNCTION, 0x3085C0, 0x302B90),
        (CURRENT_BUDDY_FUNCTION, 0x7C4610, 0x7BC2D0),
        (SET_BUDDY_HANDLE_FUNCTION, 0x4F0300, 0x4E9EF0),
        (DESTROY_BUDDY_FUNCTION, 0x486390, 0x480420),
        (TELEPORT_RESET_HAP_FUNCTION, 0x4F0FA0, 0x4EAB70),
        (PLAYER_BODY_VTABLE, 0xC61628, 0xC68948),
        (ENTITY_TERMINATE_FUNCTION, 0x746690, 0x73E6C0),
        (POSSESSED_HANDLE_VALUE, 0x12505DC, 0x124445C),
    ];
    for (address, main, compat) in expected {
        assert_eq!(address.rva(ExeVariant::Main), main);
        assert_eq!(address.rva(ExeVariant::Compat), compat);
    }
}

#[test]
fn the_object_read_addresses_are_the_ones_found_from_the_spawn_crash() {
    use automata_game::address::{
        LOADED_OBJECT_DATA_FUNCTION, OBJECT_DATA_OR_WAIT_FUNCTION,
        OBJECT_READ_IS_COMPLETE_FUNCTION, OBJECT_READ_SYSTEM_OBJECT, RELEASE_OBJECT_READ_FUNCTION,
        REQUEST_OBJECT_READ_FUNCTION, SCENE_TASK_SCHEDULER_OBJECT, SLEEP_CURRENT_TASK_FUNCTION,
        WAIT_FOR_OBJECT_READ_FUNCTION,
    };
    let expected = [
        (SLEEP_CURRENT_TASK_FUNCTION, 0x270260, 0x26AA40),
        (WAIT_FOR_OBJECT_READ_FUNCTION, 0x75EDE0, 0x756CB0),
        (OBJECT_DATA_OR_WAIT_FUNCTION, 0x84E1F0, 0x845AC0),
        (OBJECT_READ_SYSTEM_OBJECT, 0x13ECC38, 0x13E0AB8),
        (REQUEST_OBJECT_READ_FUNCTION, 0x8511B0, 0x848A50),
        (OBJECT_READ_IS_COMPLETE_FUNCTION, 0x84E950, 0x846220),
        (RELEASE_OBJECT_READ_FUNCTION, 0x850FB0, 0x848850),
        (LOADED_OBJECT_DATA_FUNCTION, 0x84E130, 0x845A00),
        (SCENE_TASK_SCHEDULER_OBJECT, 0x1029D58, 0x101DCF8),
    ];
    for (address, main, compat) in expected {
        assert_eq!(address.rva(ExeVariant::Main), main);
        assert_eq!(address.rva(ExeVariant::Compat), compat);
    }
}

#[test]
fn the_addresses_the_second_review_read_from_both_exes_are_picked_per_variant() {
    use automata_game::address::{
        ACTION_UPDATE_FUNCTION, CAMERA_MODE_REQUEST_FLAG_WORD_VALUE, CAMERA_OBJECT,
        PARTNER_WALK_IN_STATE_3B_FUNCTION, SET_BUDDY_AI_TYPE_FUNCTION, START_ANIMATION_FUNCTION,
        UI_BOOKKEEPING_WORD_VALUE,
    };
    let expected = [
        (SET_BUDDY_AI_TYPE_FUNCTION, 0x4E9DD0, 0x4E3BF0),
        (PARTNER_WALK_IN_STATE_3B_FUNCTION, 0x4EA070, 0x4E3E90),
        (START_ANIMATION_FUNCTION, 0x433C70, 0x4464C0),
        (ACTION_UPDATE_FUNCTION, 0x42C530, 0x43EE70),
        (CAMERA_OBJECT, 0x1020BB0, 0x1014A30),
        (CAMERA_MODE_REQUEST_FLAG_WORD_VALUE, 0x1029B04, 0x101D984),
    ];
    for (address, main, compat) in expected {
        assert_eq!(address.rva(ExeVariant::Main), main);
        assert_eq!(address.rva(ExeVariant::Compat), compat);
    }
    for variant in ExeVariant::ALL {
        assert_eq!(
            CAMERA_MODE_REQUEST_FLAG_WORD_VALUE.rva(variant),
            UI_BOOKKEEPING_WORD_VALUE.rva(variant) + 4
        );
    }
}

#[test]
fn the_start_readings_and_slot_calls_are_picked_per_variant() {
    use automata_game::address::{
        APPLY_LOADED_STORY_FUNCTION, CURRENT_PHASE_ID_VALUE, CURRENT_SUB_PHASE_NAME_VALUE,
        GAME_PAUSE_VALUE, NAME_RELEASE_FUNCTION, SAVE_LOAD_FAILED_FUNCTION,
        SCENE_LOADED_FLAG_VALUE,
    };
    let expected = [
        (SCENE_LOADED_FLAG_VALUE, 0x176E010, 0x1761E30),
        (CURRENT_SUB_PHASE_NAME_VALUE, 0xF64AA8, 0xF58A98),
        (SAVE_LOAD_FAILED_FUNCTION, 0x9C61F0, 0x9BD940),
        (APPLY_LOADED_STORY_FUNCTION, 0x9C6B60, 0x9BE2B0),
        (NAME_RELEASE_FUNCTION, 0x150C60, 0x150A20),
        (GAME_PAUSE_VALUE, 0x11EAC2C, 0x11E0D5C),
    ];
    for (address, main, compat) in expected {
        assert_eq!(address.rva(ExeVariant::Main), main);
        assert_eq!(address.rva(ExeVariant::Compat), compat);
    }
    for variant in ExeVariant::ALL {
        assert_eq!(
            CURRENT_SUB_PHASE_NAME_VALUE.rva(variant),
            CURRENT_PHASE_ID_VALUE.rva(variant) + 8
        );
    }
}

fn rip_target(
    function_rva: u32,
    bytes: &[u8],
    operand_at: usize,
    next_instruction_at: usize,
) -> u32 {
    let displacement = u32::from_le_bytes(bytes[operand_at..operand_at + 4].try_into().unwrap());
    function_rva + next_instruction_at as u32 + displacement
}

#[test]
fn both_compat_byte_shapes_use_the_same_save_manager_state() {
    use automata_game::address::{
        APPLY_LOADED_STORY_FUNCTION, COMPAT_APPLY_LOADED_STORY_FUNCTION_BYTES,
        COMPAT_SAVE_LOAD_FAILED_FUNCTION_BYTES, SAVE_LOAD_FAILED_FUNCTION,
    };
    let failed = SAVE_LOAD_FAILED_FUNCTION.rva(ExeVariant::Compat);
    let apply = APPLY_LOADED_STORY_FUNCTION.rva(ExeVariant::Compat);
    let state = rip_target(failed, &COMPAT_SAVE_LOAD_FAILED_FUNCTION_BYTES, 4, 8);
    assert_eq!(state, 0x14160E0);
    let shape = &COMPAT_APPLY_LOADED_STORY_FUNCTION_BYTES;
    assert_eq!(rip_target(apply, shape, 2, 7), state);
    assert_eq!(rip_target(apply, shape, 14, 22), state);
    assert_eq!(rip_target(apply, shape, 26, 34), state + 4);
    assert_eq!(shape[18..22], [4, 0, 0, 0]);
    assert_eq!(shape[30..34], [2, 0, 0, 0]);
    assert_eq!(shape.last(), Some(&0xC3));
}

#[test]
fn two_different_exes_get_one_line_naming_both() {
    use automata_game::exe::exe_difference_line;
    assert_eq!(
        exe_difference_line(ExeVariant::Main, ExeVariant::Main),
        None
    );
    let line = exe_difference_line(ExeVariant::Main, ExeVariant::Compat).unwrap();
    assert!(line.contains("the main exe"));
    assert!(line.contains("the compat exe"));
    let other = exe_difference_line(ExeVariant::Compat, ExeVariant::Main).unwrap();
    assert!(other.starts_with("net: this game is the compat exe"));
}

#[test]
fn the_pod_shot_addresses_are_the_ones_read_from_both_exes() {
    use automata_game::address::{CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION, POD_SHOT_UPDATE_FUNCTION};
    let expected = [
        (POD_SHOT_UPDATE_FUNCTION, 0x48A9D0, 0x484A60),
        (CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION, 0x184240, 0x183A80),
    ];
    for (address, main, compat) in expected {
        assert_eq!(address.rva(ExeVariant::Main), main);
        assert_eq!(address.rva(ExeVariant::Compat), compat);
    }
}

#[test]
fn the_phase_request_readings_are_picked_per_variant() {
    use automata_game::address::{
        PENDING_PHASE_ID_VALUE, PENDING_SUB_PHASE_NAME_VALUE, PHASE_JUMP_FLAG_WORD_VALUE,
        PHASE_SYSTEM_STATE_VALUE, REQUESTED_PHASE_ID_VALUE,
    };
    let expected = [
        (PHASE_SYSTEM_STATE_VALUE, 0xF64A9C, 0xF58A8C),
        (REQUESTED_PHASE_ID_VALUE, 0xF64AF0, 0xF58AE0),
        (PENDING_SUB_PHASE_NAME_VALUE, 0xF64B20, 0xF58B10),
        (PHASE_JUMP_FLAG_WORD_VALUE, 0x1029B30, 0x101D9B0),
    ];
    for (address, main, compat) in expected {
        assert_eq!(address.rva(ExeVariant::Main), main);
        assert_eq!(address.rva(ExeVariant::Compat), compat);
    }
    for variant in ExeVariant::ALL {
        assert_eq!(
            PENDING_SUB_PHASE_NAME_VALUE.rva(variant),
            PENDING_PHASE_ID_VALUE.rva(variant) + 8
        );
    }
}

#[test]
fn the_event_player_addresses_are_picked_per_variant() {
    use automata_game::address::{
        EVENT_END_HOOK_FUNCTION, EVENT_LOAD_AND_PLAY_FUNCTION, EVENT_LOAD_FUNCTION,
        EVENT_RELEASE_FUNCTION, EVENT_SKIP_FUNCTION, EVENT_SLOT_TABLE, EVENT_START_HOOK_FUNCTION,
        GAME_PAUSE_VALUE, PHASE_EVENT_ACTION_VTABLE, UI_BOOKKEEPING_WORD_VALUE,
    };
    let expected = [
        (EVENT_START_HOOK_FUNCTION, 0x7A7780, 0x79F560),
        (EVENT_END_HOOK_FUNCTION, 0x7A7D70, 0x79FB50),
        (EVENT_SKIP_FUNCTION, 0x7A8260, 0x7A0040),
        (EVENT_LOAD_AND_PLAY_FUNCTION, 0x15A560, 0x15A320),
        (EVENT_LOAD_FUNCTION, 0x15A4E0, 0x15A2A0),
        (EVENT_RELEASE_FUNCTION, 0x15A5B0, 0x15A370),
        (EVENT_SLOT_TABLE, 0x1775E30, 0x1769C50),
        (PHASE_EVENT_ACTION_VTABLE, 0xC40758, 0xC36718),
        (GAME_PAUSE_VALUE, 0x11EAC2C, 0x11E0D5C),
        (UI_BOOKKEEPING_WORD_VALUE, 0x1029B00, 0x101D980),
    ];
    for (address, main, compat) in expected {
        assert_eq!(address.rva(ExeVariant::Main), main);
        assert_eq!(address.rva(ExeVariant::Compat), compat);
    }
}

#[test]
fn the_flight_unit_calls_are_the_ones_of_the_flight_unit_ledger() {
    use automata_game::address::{
        END_PHASE_EVENT_FLIGHT_UNIT_FUNCTION, ENTITY_AS_FLIGHT_UNIT_FUNCTION,
        SET_FLIGHT_MODE_FIGHTER_FUNCTION, SET_FLIGHT_MODE_ROBOT_FUNCTION,
        START_PHASE_EVENT_FLIGHT_UNIT_FUNCTION,
    };
    let both = |address: GameAddress| {
        (
            address.rva(ExeVariant::Main),
            address.rva(ExeVariant::Compat),
        )
    };
    assert_eq!(
        both(START_PHASE_EVENT_FLIGHT_UNIT_FUNCTION),
        (0x4F8F30, 0x4F2A10)
    );
    assert_eq!(
        both(END_PHASE_EVENT_FLIGHT_UNIT_FUNCTION),
        (0x4891D0, 0x483260)
    );
    assert_eq!(both(SET_FLIGHT_MODE_FIGHTER_FUNCTION), (0x4E65A0, 0x4E0420));
    assert_eq!(both(SET_FLIGHT_MODE_ROBOT_FUNCTION), (0x4E6630, 0x4E04B0));
    assert_eq!(both(ENTITY_AS_FLIGHT_UNIT_FUNCTION), (0x45A330, 0x4545B0));
}

#[test]
fn the_story_step_addresses_are_the_ones_of_the_story_states_page() {
    use automata_game::address::{
        FIND_STORY_ACTION_BY_ID_FUNCTION, RESET_ACTION_FUNCTION, SET_ACTION_STATE_BY_HASH_FUNCTION,
        SET_ACTION_STATE_FUNCTION, STORY_HOST_OBJECT,
    };
    for (address, main, compat) in [
        (SET_ACTION_STATE_FUNCTION, 0x3E_C650, 0x3E_69C0),
        (SET_ACTION_STATE_BY_HASH_FUNCTION, 0x35_6180, 0x35_05F0),
        (RESET_ACTION_FUNCTION, 0x3E_6800, 0x3E_0B70),
        (FIND_STORY_ACTION_BY_ID_FUNCTION, 0x3D_7F80, 0x3D_2280),
        (STORY_HOST_OBJECT, 0xFC_A888, 0xFB_E748),
    ] {
        assert_eq!(address.rva(ExeVariant::Main), main);
        assert_eq!(address.rva(ExeVariant::Compat), compat);
    }
}
