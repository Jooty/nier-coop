use automata_game::address::PAD_INPUT_RECORDS;
use automata_game::exe::ExeVariant;
use automata_game::input_block::{ClearSpan, spans_to_clear};

#[test]
fn a_closed_menu_clears_nothing() {
    for variant in ExeVariant::ALL {
        assert!(spans_to_clear(false, variant).is_empty());
    }
}

#[test]
fn an_open_menu_clears_the_four_pad_records_of_the_variant() {
    assert_eq!(
        spans_to_clear(true, ExeVariant::Main),
        vec![ClearSpan {
            rva: PAD_INPUT_RECORDS.main,
            len: 4 * 0x38
        }]
    );
    assert_eq!(
        spans_to_clear(true, ExeVariant::Compat),
        vec![ClearSpan {
            rva: PAD_INPUT_RECORDS.compat,
            len: 4 * 0x38
        }]
    );
}

#[test]
fn the_pad_records_sit_at_the_noted_addresses() {
    assert_eq!(PAD_INPUT_RECORDS.main, 0x13F_CEA0);
    assert_eq!(PAD_INPUT_RECORDS.compat, 0x13F_0D20);
}
