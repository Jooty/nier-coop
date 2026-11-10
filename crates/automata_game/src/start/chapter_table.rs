use crate::start::{
    CHAPTER_HEADER_READ_LEN, CHAPTER_RECORD_SIZE, ChapterRecord, MAX_CHAPTER_RECORDS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChapterTableHeader {
    pub records_offset: u32,
    pub count: u16,
}

impl ChapterTableHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < CHAPTER_HEADER_READ_LEN {
            return Err("the chapter data header is too short".to_owned());
        }
        let header = Self {
            records_offset: u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| "header")?),
            count: u16::from_le_bytes(bytes[0x10..0x12].try_into().map_err(|_| "header")?),
        };
        if header.count == 0 || usize::from(header.count) > MAX_CHAPTER_RECORDS {
            return Err(format!(
                "the chapter data claims {} records, expected 1 to {MAX_CHAPTER_RECORDS}",
                header.count
            ));
        }
        Ok(header)
    }

    pub fn records_len(self) -> usize {
        usize::from(self.count) * CHAPTER_RECORD_SIZE
    }
}

pub fn parse_records(
    header: ChapterTableHeader,
    bytes: &[u8],
) -> Result<Vec<ChapterRecord>, String> {
    if bytes.len() < header.records_len() {
        return Err("the chapter records are shorter than the header says".to_owned());
    }
    (0..usize::from(header.count))
        .map(|index| {
            let start = index * CHAPTER_RECORD_SIZE;
            ChapterRecord::parse(&bytes[start..start + CHAPTER_RECORD_SIZE])
                .ok_or_else(|| "a chapter record is short".to_owned())
        })
        .collect()
}
