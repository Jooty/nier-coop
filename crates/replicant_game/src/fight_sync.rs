mod engine_slots;
mod guest;
mod host;
mod object_calls;
mod own_calls;
mod player_waits;
mod radio_waits;
mod replayer;
mod requests;
mod scripted_camera;
mod start_entry;
mod stream;
mod stream_stall;
mod table;
mod tape;
mod window;

pub use engine_slots::{
    callback_registry_kind_address, fired_registry_index, is_player_tested_callback_kind,
    is_registered_callback_kind, pop_source, script_slot_state,
};
pub use guest::{
    GuestArrival, GuestFightStream, GuestNotice, GuestOpen, GuestPop, GuestRadioWaitCall,
    GuestReceived, GuestRestart, GuestTapeRead, RefusedRequestStep, RestartWait,
};
pub use host::{HostFightStream, HostRequestOut, HostStreamOut, HostTapeOut};
pub use object_calls::{ArmedBy, ObjectArmedCalls, is_pop_by_this_player, queued_call_name};
pub use own_calls::{
    EarlierQueuedCalls, OwnQueuedCall, QueuedCallNotice, counted_wait_ms, is_our_queued_call,
};
pub use player_waits::{PlayerWaitNative, format_player_wait_release, player_wait_ends};
pub use radio_waits::{
    GuestRadioWaits, HostRadioWaits, RadioWaitContext, RadioWaitMark, RadioWaitPoll,
    RadioWaitRelease, RadioWaitRun, format_radio_wait_release,
};
pub use replayer::{Arrival, InFlight, PopSource, PopVerdict, Replayer, SlotState};
pub use requests::{
    HostPop, HostRequests, QueueRefusal, RequestAnswer, RequestCall, RequestReceipt,
    SceneHoldChange,
};
pub use scripted_camera::{
    GuestCameraFollow, GuestCameraStep, GuestCameraView, HostCameraSampler,
    RESET_FREE_CAMERA_DIRECTION_WITH_ACTOR_GROUP_NATIVE_RVA, SET_FIXED_CAMERA_NATIVE_RVA,
    SET_FREE_CAMERA_DIRECTION_FIXED_NATIVE_RVA, SET_FREE_CAMERA_DISTANCE_OFFSET_NATIVE_RVA,
    ScriptedCameraNative, angle_between_degrees, camera_call_line, direction_text,
    forward_of_world_matrix, guest_camera_step_line, look_angles_degrees, read_native_numbers,
    settled_camera_line,
};
pub use start_entry::{StartEntry, StartStreamPop};
pub use stream::{StreamEntry, StreamRecorder};
pub use stream_stall::{
    RadioState, StreamStall, StreamStallWatch, format_radio_line_under_menu, format_radio_state,
    format_stream_stall,
};
pub use table::{
    BASE_PLAYER_LOCAL_HANDLERS, BASE_PLAYER_LOCAL_MESSAGES, FIGHT_WINDOWS,
    REQUEST_REGISTRATION_NATIVES, TAPED_NATIVES, TRAP_PRIVATE_GAME_FLAGS,
};
pub use tape::{GuestTape, HostTape, ResultWrite, TapeKey, TapeRead, TapeValue, TapedResult};
pub use window::{
    FightWindow, FightWindowData, WindowArming, WindowCloseCause, WindowEvent, WindowState,
    fight_windows_of_pack, is_request_registration_native, is_taped_native, pack_windows_line,
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
pub const OWN_POP_HELD_FOR_HOST_COPY_MS: u64 = 3_000;
pub const REFUSED_REQUEST_RESEND_AFTER_MS: u64 = 1_000;
pub const REFUSED_REQUEST_RESENDS_BEFORE_RUNNING_HERE: u32 = 3;
pub const HOST_ENTRY_END_WAIT_LIMIT_MS: u64 = 10_000;
pub const HOST_ENTRY_END_WAIT_LOGGED_FROM_MS: u64 = 1_000;
pub const PARTNER_RESTART_EXPECTED_MS: u64 = 5_000;
pub const ONE_RESTART_SIGNALS_SPAN_MS: u64 = 3_000;
pub const LUA_STEP_RVA: u32 = 0x3E_FF30;
pub const CAMERA_SETTLE_MS: u64 = 600;
pub const CAMERA_ANGLE_MATCH_DEGREES: f32 = 8.0;
pub const GUEST_CAMERA_WAIT_AFTER_RECEIPT_MS: u64 = 300;
pub const FIGHT_CAMERA_ANGLE_KEEP_MS: u64 = 120_000;
pub const SNOW_CAMERA_INDEX: i32 = 0;
pub const GUEST_SET_DIRECTION_BLEND_SECONDS: f64 = 0.0;
pub const GUEST_SET_DIRECTION_EASE: [f64; 3] = [0.1, 1.0, 0.1];
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
pub const QUEUED_CALL_GIVEN_UP_AFTER_MS: u64 = 30_000;
pub const MAX_WAITING_GUEST_REQUESTS: usize = 32;
pub const NEXT_WINDOW_HELD_AT_MOST_MS: u64 = 30_000;
pub const MAX_HELD_NEXT_WINDOW_MESSAGES: usize = 1_024;
pub const RADIO_WAIT_BEHIND_STREAM_GRACE_MS: u64 = 500;
pub const PLAYER_WAIT_BEHIND_STREAM_GRACE_MS: u64 = 3_000;
pub const RADIO_WAIT_MARK_SLACK_FOR_AN_ADVANCING_LINE_MS: u64 = 1_000;
pub const MAX_RADIO_WAIT_MARKS_KEPT: usize = 64;
pub const STREAM_STALL_LOGGED_FROM_MS: u64 = 5_000;
pub const STREAM_STALL_LOGGED_EVERY_MS: u64 = 5_000;
pub const RADIO_MANAGER_RVA: u32 = 0x4B8_72F0;
pub const RADIO_BUSY_BYTE_OFFSET: u64 = 0xC_D5C0;
pub const RADIO_ACTIVE_BYTE_OFFSET: u64 = 0xC_D5C2;
pub const RADIO_LINE_STATE_OFFSET: u64 = 0x100;
pub const RADIO_LINE_TIME_OFFSET: u64 = 0xB_CD60;
pub const RADIO_LINE_FLAG_OFFSET: u64 = 0xB_CD68;
pub const RADIO_ABORT_FLAG_WORD_RVA: u32 = 0x26E_6168;
pub const RADIO_ABORT_FLAG_BIT: u32 = 0x4;
pub const RADIO_STOP_RVA: u32 = 0x4A_B180;
pub const START_RADIO_WAIT_RVA: u32 = 0x70_A6B0;
pub const WAIT_NATIVE_WAIT_RVA: u32 = 0x70_BDA0;
pub const POLLED_NATIVE_STATE_OFFSET: u64 = 0x8;
