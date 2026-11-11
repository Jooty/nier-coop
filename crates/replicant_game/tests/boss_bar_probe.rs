use replicant_game::enemies::{
    ActorHandle, BOSS_BAR_DUMP_BYTES, boss_bar_dump_text, boss_bar_init_text, boss_bar_script_text,
    script_argument_value,
};

fn argument(tag: i32, value: &[u8; 8]) -> [u8; 16] {
    let mut raw = [0_u8; 16];
    raw[0..4].copy_from_slice(&tag.to_le_bytes());
    raw[8..16].copy_from_slice(value);
    raw
}

#[test]
fn an_int_argument_reads_its_value() {
    let mut value = [0_u8; 8];
    value[0..4].copy_from_slice(&(-3_i32).to_le_bytes());
    assert_eq!(script_argument_value(&argument(4, &value)), -3);
}

#[test]
fn a_double_argument_is_cut_to_an_int() {
    assert_eq!(
        script_argument_value(&argument(1, &7.9_f64.to_le_bytes())),
        7
    );
}

#[test]
fn any_other_argument_kind_reads_zero() {
    assert_eq!(script_argument_value(&argument(2, &[9; 8])), 0);
}

#[test]
fn the_init_line_names_handle_and_enemy() {
    let handle = ActorHandle {
        index: 12,
        generation: 3,
    };
    assert_eq!(
        boss_bar_init_text(Some(handle), Some("5:2")),
        "boss bar: init (handle 12/3, enemy 5:2)"
    );
    assert_eq!(
        boss_bar_init_text(None, None),
        "boss bar: init (handle ?, enemy ?)"
    );
}

#[test]
fn the_script_line_marks_unreadable_arguments() {
    assert_eq!(
        boss_bar_script_text(Some(4), None),
        "boss bar: script command (args 4, ?)"
    );
}

#[test]
fn the_dump_line_has_hex_for_each_part_and_marks_unreadable_ones() {
    let bytes = [0xAB_u8; BOSS_BAR_DUMP_BYTES];
    let text = boss_bar_dump_text(Some(&bytes), None, Some(&bytes));
    assert!(text.starts_with("boss bar: hud life widget +0x1d0 abab"));
    assert!(text.contains(", digi +0x2b8 unreadable, bar object abab"));
    assert!(text.ends_with(&"ab".repeat(BOSS_BAR_DUMP_BYTES)));
}
