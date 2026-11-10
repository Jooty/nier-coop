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
fn the_frame_end_and_input_update_functions_differ_per_exe() {
    use automata_game::address::{FRAME_END_FUNCTION, INPUT_UPDATE_ALL_FUNCTION};
    assert_eq!(FRAME_END_FUNCTION.rva(ExeVariant::Main), 0x24A250);
    assert_eq!(FRAME_END_FUNCTION.rva(ExeVariant::Compat), 0x2437D0);
    assert_eq!(INPUT_UPDATE_ALL_FUNCTION.rva(ExeVariant::Main), 0x86C310);
    assert_eq!(INPUT_UPDATE_ALL_FUNCTION.rva(ExeVariant::Compat), 0x863BB0);
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
