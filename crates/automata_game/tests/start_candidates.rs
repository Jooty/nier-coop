use automata_game::start::{ChapterRecord, candidates, table_log_lines};

fn record(chapter: u8, group: u8, mask: u8, phase_id: i16, name: &str) -> ChapterRecord {
    ChapterRecord {
        chapter,
        group,
        route: 0,
        hero_mask: mask,
        phase_id,
        phase_name: name.to_owned(),
    }
}

fn sample_table() -> Vec<ChapterRecord> {
    vec![
        record(0, 1, 1, 0x100, "00_30_Prologue_STG1"),
        record(1, 1, 1, 0x101, "00_31_Prologue_STG2"),
        record(2, 2, 1, 0x200, "01_10_City"),
        record(3, 2, 1, 0x201, "01_11_City_B"),
        record(4, 3, 1, 0x300, "02_10_Bunker_Hub"),
        record(5, 4, 0b10, 0x400, "03_10_Second_Only"),
        record(6, 0x11, 1, 0x500, "99_10_Conditional"),
        record(7, 5, 0b101, 0x600, "04_10_Desert"),
    ]
}

#[test]
fn only_the_first_record_of_a_group_can_be_a_candidate() {
    let found = candidates(&sample_table());
    let phases: Vec<i16> = found.iter().map(|c| c.record.phase_id).collect();
    assert!(!phases.contains(&0x201));
    assert!(phases.contains(&0x200));
}

#[test]
fn the_opening_mission_the_base_second_character_only_and_conditional_groups_are_left_out() {
    let found = candidates(&sample_table());
    let phases: Vec<i16> = found.iter().map(|c| c.record.phase_id).collect();
    assert_eq!(phases, [0x200, 0x600]);
}

#[test]
fn candidates_are_numbered_in_table_order_and_remember_their_record() {
    let found = candidates(&sample_table());
    assert_eq!(found[0].index, 0);
    assert_eq!(found[0].record_index, 2);
    assert_eq!(found[1].index, 1);
    assert_eq!(found[1].record_index, 7);
}

#[test]
fn a_group_that_comes_back_later_starts_a_new_header() {
    let table = vec![
        record(0, 2, 1, 0x20, "a"),
        record(1, 3, 1, 0x30, "b"),
        record(2, 2, 1, 0x21, "c"),
    ];
    assert_eq!(candidates(&table).len(), 3);
}

#[test]
fn an_empty_table_has_no_candidates() {
    assert!(candidates(&[]).is_empty());
}

#[test]
fn the_table_is_logged_one_line_per_record_with_the_candidate_marked() {
    let table = sample_table();
    let lines = table_log_lines(&table, &candidates(&table));
    assert_eq!(lines.len(), table.len());
    assert_eq!(
        lines[2],
        "start: chapter 2 group 2 n 2 route 0 hero mask 0x1 phase 0x200 01_10_City [candidate 0]"
    );
    assert_eq!(
        lines[3],
        "start: chapter 3 group 2 n 3 route 0 hero mask 0x1 phase 0x201 01_11_City_B"
    );
    assert!(lines[7].ends_with("[candidate 1]"));
}

#[test]
fn the_name_tests_ignore_case() {
    let table = vec![
        record(0, 2, 1, 0x100, "00_PROLOGUE_stg"),
        record(1, 3, 1, 0x300, "02_10_BUNKER_hub"),
        record(2, 4, 1, 0x400, "03_10_bunker_hub"),
        record(3, 5, 1, 0x600, "04_10_Desert"),
    ];
    let phases: Vec<i16> = candidates(&table)
        .iter()
        .map(|c| c.record.phase_id)
        .collect();
    assert_eq!(phases, [0x600]);
}

#[test]
fn candidates_that_share_a_phase_id_stay_distinct_by_chapter_and_name() {
    let table = vec![
        record(12, 2, 3, 0x100, "name_one"),
        record(14, 3, 3, 0x100, "name_two"),
    ];
    let found = candidates(&table);
    assert_eq!(found.len(), 2);
    assert_ne!(found[0].record.chapter, found[1].record.chapter);
}
