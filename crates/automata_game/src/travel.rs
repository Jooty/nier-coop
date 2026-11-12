mod come_to_sender;
mod jump_rule;
mod loaded_points;
mod partner_move;
mod teleport_request;
mod travel_machine;
mod travel_point;
mod travel_view;

pub use come_to_sender::{body_of_sender, newest_come_sender};
pub use jump_rule::{
    JumpRefusal, JumpRole, WorldReadings, decide_followed_jump, decide_local_jump, game_is_paused,
    world_refusal,
};
pub use loaded_points::{LoadedPoints, point_list_log_line};
pub use partner_move::{
    BringPartnerRefusal, GoToPartnerReadings, GoToPartnerRefusal, MoveAsker, decide_bring_partner,
    decide_go_to_partner, go_to_partner_line, move_distance,
};
pub use teleport_request::{
    IncomingTeleport, NewestRequests, TeleportRequest, handle_teleport_request, newest_requests,
};
pub use travel_machine::{JumpOrigin, TravelAction, TravelMachine, TravelStep};
pub use travel_point::TravelPoint;
pub use travel_view::TravelView;

pub const TRAVEL_POINT_CODES: [&str; 22] = [
    "ft_RC", "ft_CC", "ft_CRb", "ft_CS", "ft_DC", "ft_DI", "ft_DO", "ft_DD", "ft_AP", "ft_AP_C",
    "ft_AP_T", "ft_AP_T2", "ft_RME", "ft_RMI2", "ft_RMI", "ft_AS", "ft_PV", "ft_FC", "ft_FC2",
    "ft_FC3", "ft_SC", "ft_BK",
];
pub const TRAVEL_POINT_LABEL_PREFIX: &str = "Travel point ";

pub const FAST_TRAVEL_STATE_OFFSET: u32 = 0x8;
pub const FAST_TRAVEL_STEP_OFFSET: u32 = 0x58;
pub const FAST_TRAVEL_POINT_LIST_HEAD_OFFSET: u32 = 0x78;
pub const TRAVEL_POINT_NEXT_OFFSET: u32 = 0x190;
pub const TRAVEL_POINT_DATA_POINTER_OFFSET: u32 = 0x8;
pub const TRAVEL_POINT_DATA_FLAGS_OFFSET: u32 = 0x20;
pub const TRAVEL_POINT_CODE_NAME_OFFSET: u32 = 0xB8;
pub const INTERNED_NAME_TEXT_POINTER_OFFSET: u32 = 0x28;
pub const TRAVEL_POINT_LIST_END_MARK: u64 = u64::MAX;
pub const MAX_TRAVEL_POINTS_WALKED: usize = 512;
pub const TRAVEL_POINT_NAME_READ_LIMIT: usize = 0x40;
pub const MAX_POINT_NAMES_IN_ONE_LOG_LINE: usize = 40;

pub const PAUSED_GAME_VALUES: [i32; 4] = [2, 3, 4, 8];
pub const FAST_TRAVEL_IDLE_STATE: u32 = 0x4B7B_6E51;
pub const FAST_TRAVEL_STATE_THE_TRAVEL_CALL_RESETS_TO_IDLE: u32 = 0x2B17_FE28;
pub const FAST_TRAVEL_LAST_STEP_BEFORE_THE_JUMP: i32 = 5;
pub const FAST_TRAVEL_STEP_AT_REST: i32 = -1;
pub const BODY_STATE_THE_RESET_TELEPORT_IGNORES: i32 = 0xD3;
pub const DEATH_CHAIN_STATES: [i32; 2] = [0x35, 0x36];

pub const POINT_LIST_READ_INTERVAL_MS: u64 = 500;
pub const POINT_LIST_WANTED_AFTER_THE_MENU_ASKED_MS: u64 = 1_000;
pub const MOVE_TO_PARTNER_MIN_INTERVAL_MS: u64 = 2_000;
pub const SCENE_LEAVE_LIMIT_MS: u64 = 15_000;
pub const SCENE_LEAVE_LIMIT_WHILE_THE_TRANSPORT_STEPS_MS: u64 = 60_000;
pub const FOLLOWED_JUMP_WAIT_MS: u64 = 15_000;
pub const JUMP_ARRIVAL_LIMIT_MS: u64 = 300_000;
pub const ARRIVAL_SHOWN_AS_CURRENT_MS: u64 = 10_000;
