use rc_proto::MAX_SCRIPT_ROLL_ORDINAL;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RollOrdinals {
    run: Option<(u64, u64)>,
    next: u16,
}

impl RollOrdinals {
    pub const fn new() -> Self {
        Self { run: None, next: 0 }
    }

    pub fn next_for(&mut self, call_hash: u64, started_ms: u64) -> u16 {
        if self.run != Some((call_hash, started_ms)) {
            self.run = Some((call_hash, started_ms));
            self.next = 0;
        }
        let ordinal = self.next;
        self.next = (self.next + 1).min(MAX_SCRIPT_ROLL_ORDINAL);
        ordinal
    }
}
