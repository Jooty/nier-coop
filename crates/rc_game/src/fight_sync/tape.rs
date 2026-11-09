use std::collections::{HashMap, HashSet};

use rc_proto::FightTapeValue;

use crate::story_relay::{ARG_TAG_INT, ARG_TAG_NUMBER};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TapeValue {
    Int(i64),
    Float(f64),
    /// The native returned no value (its actor or group was not there).
    Missing,
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
    pub from_host: bool,
    pub log: Option<String>,
}

/// Host side: numbers each state-read native's result inside the handler of stream entry `seq`;
/// each one is sent to the guest as soon as it is read.
#[derive(Debug, Clone, Default)]
pub struct HostTape {
    current_seq: Option<u64>,
    call_counts: HashMap<String, u32>,
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

impl TapeValue {
    /// The value a native left in its first result slot: `result_count` from the call block,
    /// then that slot's tag and 8 payload bytes.
    pub fn from_result_slot(result_count: i32, tag: i32, payload: [u8; 8]) -> Self {
        if result_count < 1 {
            return Self::Missing;
        }
        match tag {
            ARG_TAG_INT => {
                let [a, b, c, d, ..] = payload;
                Self::Int(i64::from(i32::from_le_bytes([a, b, c, d])))
            }
            ARG_TAG_NUMBER => Self::Float(f64::from_le_bytes(payload)),
            _ => Self::Missing,
        }
    }

    /// The bytes to write over the result slot's payload, for a value of the slot's own kind.
    pub fn result_payload(self) -> Option<Vec<u8>> {
        match self {
            Self::Int(int) => i32::try_from(int)
                .ok()
                .map(|int| int.to_le_bytes().to_vec()),
            Self::Float(float) => Some(float.to_le_bytes().to_vec()),
            Self::Missing => None,
        }
    }

    pub const fn to_wire(self) -> FightTapeValue {
        match self {
            Self::Int(int) => FightTapeValue::Int(int),
            Self::Float(float) => FightTapeValue::Number(float),
            Self::Missing => FightTapeValue::Missing,
        }
    }

    pub const fn from_wire(value: FightTapeValue) -> Self {
        match value {
            FightTapeValue::Int(int) => Self::Int(int),
            FightTapeValue::Number(number) => Self::Float(number),
            FightTapeValue::Missing => Self::Missing,
        }
    }
}

impl HostTape {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin(&mut self, seq: u64) {
        self.current_seq = Some(seq);
        self.call_counts.clear();
    }

    pub fn end(&mut self) {
        self.current_seq = None;
        self.call_counts.clear();
    }

    pub const fn current_seq(&self) -> Option<u64> {
        self.current_seq
    }

    /// The numbered result to send, or `None` outside a streamed handler.
    pub fn record(&mut self, native: &str, value: TapeValue) -> Option<TapedResult> {
        let seq = self.current_seq?;
        let call_index = next_call_index(&mut self.call_counts, native);
        Some(TapedResult {
            key: TapeKey {
                seq,
                native: native.to_owned(),
                call_index,
            },
            value,
        })
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

    pub const fn current_seq(&self) -> Option<u64> {
        self.current_seq
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
                from_host: false,
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
                    from_host: true,
                    log: None,
                };
            }
            Some(taped) => format!("host taped {taped:?}, a different kind"),
            None => "the host taped nothing".to_owned(),
        };
        let first_for_this_native = self.logged.insert((seq, native.to_owned()));
        TapeRead {
            value: local,
            from_host: false,
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
        (TapeValue::Int(_), TapeValue::Int(_))
            | (TapeValue::Float(_), TapeValue::Float(_))
            | (TapeValue::Missing, TapeValue::Missing)
    )
}
