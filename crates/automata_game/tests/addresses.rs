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
