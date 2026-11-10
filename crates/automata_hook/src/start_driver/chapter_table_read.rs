use automata_game::address::CHAPTER_DATA_POINTER;
use automata_game::start::{
    CHAPTER_HEADER_READ_LEN, ChapterRecord, ChapterTableHeader, parse_records,
};

use crate::game_calls::GameCalls;

pub(super) fn read_chapter_table(calls: &GameCalls) -> Result<Vec<ChapterRecord>, String> {
    let data = calls
        .read_u64(CHAPTER_DATA_POINTER)
        .map_err(|error| format!("{error}"))?;
    if data == 0 {
        return Err("the chapter data pointer is null".to_owned());
    }
    let head = calls
        .read_bytes_at(data, CHAPTER_HEADER_READ_LEN)
        .map_err(|error| format!("{error}"))?;
    let header = ChapterTableHeader::parse(&head)?;
    let records = calls
        .read_bytes_at(
            data + u64::from(header.records_offset),
            header.records_len(),
        )
        .map_err(|error| format!("{error}"))?;
    parse_records(header, &records)
}
