use automata_game::start::{
    CHAPTER_HEADER_READ_LEN, CHAPTER_RECORD_SIZE, ChapterRecord, ChapterTableHeader, parse_records,
};

pub fn record_bytes(
    chapter: u8,
    group: u8,
    route: i32,
    hero_mask: u8,
    phase_id: i16,
    name: &str,
) -> Vec<u8> {
    let mut bytes = vec![0_u8; CHAPTER_RECORD_SIZE];
    bytes[0] = chapter;
    bytes[1] = group;
    bytes[4..8].copy_from_slice(&route.to_le_bytes());
    bytes[8] = hero_mask;
    bytes[0xC..0xE].copy_from_slice(&phase_id.to_le_bytes());
    bytes[0xE..0xE + name.len()].copy_from_slice(name.as_bytes());
    bytes
}

fn header_bytes(records_offset: u32, count: u16) -> Vec<u8> {
    let mut bytes = vec![0_u8; CHAPTER_HEADER_READ_LEN];
    bytes[8..12].copy_from_slice(&records_offset.to_le_bytes());
    bytes[0x10..0x12].copy_from_slice(&count.to_le_bytes());
    bytes
}

#[test]
fn a_record_is_read_field_by_field() {
    let record =
        ChapterRecord::parse(&record_bytes(7, 3, 1, 0b101, 0x210, "made_up_phase")).unwrap();
    assert_eq!(record.chapter, 7);
    assert_eq!(record.group, 3);
    assert_eq!(record.route, 1);
    assert_eq!(record.hero_mask, 0b101);
    assert_eq!(record.phase_id, 0x210);
    assert_eq!(record.phase_name, "made_up_phase");
}

#[test]
fn a_negative_route_and_the_longest_name_with_an_end_are_kept() {
    let longest = "n".repeat(0x1F);
    let record = ChapterRecord::parse(&record_bytes(0, 1, -1, 1, 5, &longest)).unwrap();
    assert_eq!(record.route, -1);
    assert_eq!(record.phase_name, longest);
}

#[test]
fn a_name_that_fills_the_field_without_an_end_is_refused() {
    let full = "n".repeat(0x20);
    assert_eq!(
        ChapterRecord::parse(&record_bytes(0, 1, -1, 1, 5, &full)),
        None
    );
    let mut bytes = record_bytes(1, 1, 0, 1, 0x10, "first");
    bytes.extend(record_bytes(2, 1, 0, 1, 0x20, &full));
    let header = ChapterTableHeader::parse(&header_bytes(0x40, 2)).unwrap();
    assert!(parse_records(header, &bytes).is_err());
}

#[test]
fn a_short_record_is_refused() {
    assert_eq!(ChapterRecord::parse(&[0_u8; CHAPTER_RECORD_SIZE - 1]), None);
}

#[test]
fn the_header_gives_the_offset_and_the_count() {
    let header = ChapterTableHeader::parse(&header_bytes(0x40, 3)).unwrap();
    assert_eq!(header.records_offset, 0x40);
    assert_eq!(header.count, 3);
    assert_eq!(header.records_len(), 3 * CHAPTER_RECORD_SIZE);
}

#[test]
fn a_header_with_no_records_or_too_many_is_refused() {
    assert!(ChapterTableHeader::parse(&header_bytes(0x40, 0)).is_err());
    assert!(ChapterTableHeader::parse(&header_bytes(0x40, 5000)).is_err());
    assert!(ChapterTableHeader::parse(&[0_u8; 4]).is_err());
}

#[test]
fn all_records_of_the_table_are_parsed_in_order() {
    let mut bytes = record_bytes(1, 1, 0, 1, 0x10, "first");
    bytes.extend(record_bytes(2, 1, 0, 1, 0x20, "second"));
    let header = ChapterTableHeader::parse(&header_bytes(0x40, 2)).unwrap();
    let records = parse_records(header, &bytes).unwrap();
    let names: Vec<&str> = records.iter().map(|r| r.phase_name.as_str()).collect();
    assert_eq!(names, ["first", "second"]);
}

#[test]
fn records_shorter_than_the_header_says_are_refused() {
    let bytes = record_bytes(1, 1, 0, 1, 0x10, "only");
    let header = ChapterTableHeader::parse(&header_bytes(0x40, 2)).unwrap();
    assert!(parse_records(header, &bytes).is_err());
}
