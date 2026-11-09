mod replayer;
mod stream;
mod table;
mod tape;
mod window;

pub use replayer::{Arrival, InFlight, PopSource, PopVerdict, Replayer, SlotState};
pub use stream::{StreamEntry, StreamRecorder};
pub use table::{
    BASE_PLAYER_LOCAL_HANDLERS, BASE_PLAYER_LOCAL_MESSAGES, FIGHT_WINDOWS,
    REQUEST_REGISTRATION_NATIVES, TAPED_NATIVES,
};
pub use tape::{GuestTape, HostTape, TapeKey, TapeRead, TapeValue, TapedResult};
pub use window::{
    FightWindow, FightWindowData, WindowCloseCause, WindowEvent, WindowState,
    fight_windows_of_pack, is_request_registration_native, is_taped_native,
};

pub const PLAYER_LOCAL_CALLBACK_NAMES: [&str; 2] = ["LIB_GameOver", "SQ_Fishing_Radio"];
