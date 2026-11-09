mod engine_slots;
mod guest;
mod host;
mod object_calls;
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
    GuestTapeRead, RestartWait,
};
pub use host::{HostFightStream, HostRequestOut, HostStreamOut, HostTapeOut};
pub use object_calls::{ArmedBy, ObjectArmedCalls, is_pop_by_this_player, queued_call_name};
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
pub const HOST_ENTRY_END_WAIT_LIMIT_MS: u64 = 10_000;
pub const HOST_ENTRY_END_WAIT_LOGGED_FROM_MS: u64 = 1_000;
pub const PARTNER_RESTART_EXPECTED_MS: u64 = 5_000;
pub const ONE_RESTART_SIGNALS_SPAN_MS: u64 = 3_000;
pub const LUA_STEP_RVA: u32 = 0x3E_FF30;
pub const TAPED_NATIVE_RVAS: [(&str, u32); 10] = [
    ("_GetRandI", 0x6E_BE80),
    ("_GetRandF", 0x6E_BDC0),
    ("_GetActorLife", 0x6E_9FA0),
    ("_GetActorLifeRate", 0x6E_A020),
    ("_GetUniqueActorMaxLife", 0x6E_C5A0),
    ("_GetActorCount", 0x6E_9C30),
    ("_GetActorCountGroups", 0x6E_9C80),
    ("_GetActorNumInScriptArea", 0x6E_A150),
    ("_GetNearPlayerEnemyNum", 0x6E_B410),
    ("_GetGlobalTimeCount", 0x6E_B0B0),
];
pub const TAPED_NATIVES_HOOKED_BY_SCRIPT_ROLLS: [&str; 2] = ["_GetRandI", "_GetRandF"];
pub const HOST_TAPE_LOGGED_KEYS_KEPT: usize = 256;
pub const HOST_TAPE_LOGGED_SEQS_BACK: u64 = 8;
pub const QUEUED_CALL_NAME_OFFSET: u64 = 0x4;
pub const QUEUED_CALL_NAME_CAPACITY: usize = 0x40;
