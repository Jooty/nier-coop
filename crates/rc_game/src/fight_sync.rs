mod replayer;
mod stream;
mod tape;

pub use replayer::{Arrival, InFlight, PopSource, PopVerdict, Replayer, SlotState};
pub use stream::{StreamEntry, StreamRecorder};
pub use tape::{GuestTape, HostTape, TapeKey, TapeRead, TapeValue, TapedResult};

pub const PLAYER_LOCAL_CALLBACK_NAMES: [&str; 2] = ["LIB_GameOver", "SQ_Fishing_Radio"];
