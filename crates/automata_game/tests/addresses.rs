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
