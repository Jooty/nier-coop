use rc_game::position_log::{
    POSITION_NATIVES, describe_argument, describe_arguments, native_argument_offset,
    native_argument_value_offset,
};
use rc_game::story_jump::{
    NATIVE_ARG_INTEGER_TYPE_TAG, NATIVE_ARG_REAL_TYPE_TAG, SET_PLAYER_POSITION_NATIVE_RVA,
};

#[test]
fn the_three_position_natives_have_distinct_addresses() {
    let mut rvas: Vec<u32> = POSITION_NATIVES.iter().map(|native| native.rva).collect();
    rvas.sort_unstable();
    rvas.dedup();
    assert_eq!(rvas.len(), 3);
    assert!(rvas.contains(&SET_PLAYER_POSITION_NATIVE_RVA));
}

#[test]
fn an_integer_argument_reads_the_low_half() {
    let bits = 0xFFFF_FFFF_0000_0004_u64;
    assert_eq!(
        describe_argument(NATIVE_ARG_INTEGER_TYPE_TAG, bits).as_deref(),
        Some("4")
    );
}

#[test]
fn a_negative_integer_argument_keeps_its_sign() {
    let bits = u64::from((-1_i32) as u32);
    assert_eq!(
        describe_argument(NATIVE_ARG_INTEGER_TYPE_TAG, bits).as_deref(),
        Some("-1")
    );
}

#[test]
fn a_real_argument_prints_one_decimal() {
    let bits = 146.84_f64.to_bits();
    assert_eq!(
        describe_argument(NATIVE_ARG_REAL_TYPE_TAG, bits).as_deref(),
        Some("146.8")
    );
}

#[test]
fn an_unknown_tag_has_no_value() {
    assert_eq!(describe_argument(9, 0), None);
}

#[test]
fn arguments_join_with_commas_and_mark_unknown_tags() {
    let arguments = [
        (NATIVE_ARG_INTEGER_TYPE_TAG, 7),
        (NATIVE_ARG_REAL_TYPE_TAG, 2.0_f64.to_bits()),
        (0, 0),
    ];
    assert_eq!(describe_arguments(&arguments), "7, 2.0, ?tag0");
}

#[test]
fn the_value_sits_after_the_tag_in_each_argument_slot() {
    assert!(native_argument_value_offset(0) > native_argument_offset(0));
    assert!(native_argument_offset(1) > native_argument_value_offset(0));
}
