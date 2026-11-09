mod engine_slots;
mod guest;
mod host;
mod replayer;
mod requests;
mod stream;
mod table;
mod tape;
mod window;

pub use engine_slots::{
    callback_registry_kind_address, fired_registry_index, is_player_tested_callback_kind,
    is_registered_callback_kind, pop_source, script_slot_state,
};
pub use guest::{
    GuestArrival, GuestFightStream, GuestNotice, GuestOpen, GuestPop, GuestReceived, GuestRestart,
    RestartWait,
};
pub use host::{HostFightStream, HostRequestOut, HostStreamOut};
pub use replayer::{Arrival, InFlight, PopSource, PopVerdict, Replayer, SlotState};
pub use requests::{HostPop, HostRequests, RequestCall, RequestReceipt};
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

pub const CALLBACK_REGISTRY_KIND_FIRST_OFFSET: u64 = 0x4;
pub const CALLBACK_REGISTRY_ARGUMENTS_FIRST_OFFSET: u64 = 0x70;
pub const CALLBACK_REGISTRY_ENTRY_STRIDE: u64 = 0x7C8;
pub const CALLBACK_REGISTRY_ENTRY_COUNT: u64 = 256;
pub const PHASE_SCRIPT_MANAGER_STATE_OFFSET: u64 = 0x1718;
pub const SCRIPT_SLOT_WAITING_TO_POP_STATES: [i32; 2] = [0x6C8, 0x6CD];
pub const SCRIPT_SLOT_STEPPING_HELD_STATE: i32 = 0x6AD;
pub const PARTNER_CALL_POP_EXPECTED_MS: u64 = 5_000;
pub const PLAYER_TESTED_CALLBACK_KINDS: [i32; 7] = [4, 5, 6, 7, 8, 0xA, 0x11];
pub const REQUEST_ANSWER_EXPECTED_MS: u64 = 5_000;
pub const SAME_POP_ON_BOTH_GAMES_SPAN_MS: u64 = 3_000;
pub const PARTNER_RESTART_EXPECTED_MS: u64 = 5_000;
pub const ONE_RESTART_SIGNALS_SPAN_MS: u64 = 3_000;
