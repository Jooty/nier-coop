use crate::exe::{ExeFacts, ExeVariant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExeIdentity {
    pub name: &'static str,
    pub variant: ExeVariant,
    pub file_size: u64,
    pub time_date_stamp: u32,
    pub sha256_hex: &'static str,
}

impl ExeIdentity {
    pub fn matches(&self, facts: &ExeFacts) -> bool {
        self.file_size == facts.file_size
            && self.time_date_stamp == facts.time_date_stamp
            && self.sha256_hex.eq_ignore_ascii_case(&facts.sha256_hex)
    }
}
