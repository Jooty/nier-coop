#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExeFacts {
    pub file_size: u64,
    pub time_date_stamp: u32,
    pub sha256_hex: String,
}
