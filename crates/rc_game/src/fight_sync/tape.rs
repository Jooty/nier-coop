use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TapeValue {
    Int(i64),
    Float(f32),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TapeKey {
    pub seq: u64,
    pub native: String,
    pub call_index: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TapedResult {
    pub key: TapeKey,
    pub value: TapeValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TapeRead {
    pub value: TapeValue,
    pub log: Option<String>,
}

/// Host side: records each state-read native's result inside the handler of stream entry `seq`.
#[derive(Debug, Clone, Default)]
pub struct HostTape {
    current_seq: Option<u64>,
    call_counts: HashMap<String, u32>,
    recorded: Vec<TapedResult>,
}

/// Guest side: inside a replayed handler a read returns the host's value for the same
/// (seq, native, call index); a missing or mismatched one returns the local value and logs once.
#[derive(Debug, Clone, Default)]
pub struct GuestTape {
    results: HashMap<TapeKey, TapeValue>,
    current_seq: Option<u64>,
    call_counts: HashMap<String, u32>,
    logged: HashSet<(u64, String)>,
}

impl HostTape {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin(&mut self, seq: u64) {
        self.current_seq = Some(seq);
        self.call_counts.clear();
    }

    pub fn end(&mut self) -> Vec<TapedResult> {
        self.current_seq = None;
        self.call_counts.clear();
        std::mem::take(&mut self.recorded)
    }

    pub fn record(&mut self, native: &str, value: TapeValue) {
        let Some(seq) = self.current_seq else {
            return;
        };
        let call_index = next_call_index(&mut self.call_counts, native);
        self.recorded.push(TapedResult {
            key: TapeKey {
                seq,
                native: native.to_owned(),
                call_index,
            },
            value,
        });
    }
}

impl GuestTape {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(&mut self, results: Vec<TapedResult>) {
        for result in results {
            self.results.insert(result.key, result.value);
        }
    }

    pub fn begin(&mut self, seq: u64) {
        self.current_seq = Some(seq);
        self.call_counts.clear();
    }

    pub fn end(&mut self) {
        if let Some(seq) = self.current_seq.take() {
            self.results.retain(|key, _| key.seq > seq);
        }
        self.call_counts.clear();
    }

    pub fn clear(&mut self) {
        self.results.clear();
        self.current_seq = None;
        self.call_counts.clear();
        self.logged.clear();
    }

    pub fn read(&mut self, native: &str, local: TapeValue) -> TapeRead {
        let Some(seq) = self.current_seq else {
            return TapeRead {
                value: local,
                log: None,
            };
        };
        let call_index = next_call_index(&mut self.call_counts, native);
        let key = TapeKey {
            seq,
            native: native.to_owned(),
            call_index,
        };
        let problem = match self.results.get(&key) {
            Some(taped) if same_kind(*taped, local) => {
                return TapeRead {
                    value: *taped,
                    log: None,
                };
            }
            Some(taped) => format!("host taped {taped:?}, a different kind"),
            None => "the host taped nothing".to_owned(),
        };
        let first_for_this_native = self.logged.insert((seq, native.to_owned()));
        TapeRead {
            value: local,
            log: first_for_this_native.then(|| {
                format!(
                    "fight tape: {native} call {call_index} in seq {seq}: {problem}, local value {local:?} used"
                )
            }),
        }
    }
}

fn next_call_index(call_counts: &mut HashMap<String, u32>, native: &str) -> u32 {
    let count = call_counts.entry(native.to_owned()).or_insert(0);
    let index = *count;
    *count += 1;
    index
}

fn same_kind(left: TapeValue, right: TapeValue) -> bool {
    matches!(
        (left, right),
        (TapeValue::Int(_), TapeValue::Int(_)) | (TapeValue::Float(_), TapeValue::Float(_))
    )
}
