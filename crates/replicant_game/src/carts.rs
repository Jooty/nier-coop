mod guest_follow;
mod host_watch;
mod scan;
mod state;
mod text;

pub use guest_follow::{CartFollowStep, GuestCartFollow, OwnCartFacts};
pub use host_watch::{CartRideEvent, CartRideFacts, HostRideWatch};
pub use scan::{CartSighting, cart_is_live, match_cart, read_cart_state, scan_cart_actors};
pub use state::{CartState, cart_state, parked_state_after, run_state_from};
pub use text::{
    cart_follow_step_text, cart_ride_event_text, carts_here_text, guest_ride_refused_text,
    partner_wants_to_ride_text, riding_along_text,
};

pub const CART_VTABLE_RVA: u32 = 0x00B0_7068;
pub const CART_STATE_OFFSET: u64 = 0x1_4D84;
pub const CART_SET_STATE_FUNCTION_RVA: u32 = 0x0016_1070;
pub const CART_MESSAGE_FUNCTION_RVA: u32 = 0x0016_8430;
pub const CART_BOARD_MESSAGE_ID: u32 = 0x11;
pub const PLAYER_ON_CART_FLAG_OFFSET: u64 = 0x6_B74C;
pub const FIRST_PARKED_CART_STATE: u8 = 1;
pub const LAST_PARKED_CART_STATE: u8 = 4;
pub const FIRST_RUNNING_CART_STATE: u8 = 5;
pub const LAST_RUNNING_CART_STATE: u8 = 8;
pub const CART_SCAN_INTERVAL_MS: u64 = 100;
pub const CART_MATCH_UNITS: f32 = 8.0;
pub const CART_RIDER_LIFT_UNITS: f32 = 0.3;
pub const CART_RIDER_REACH_UNITS: f32 = 6.0;
pub const CART_BOARD_SETTLE_MS: u64 = 400;
pub const CART_BOARD_GIVE_UP_MS: u64 = 6_000;
pub const CART_END_GRACE_MS: u64 = 3_000;
pub const CART_END_PULL_DELAY_MS: u64 = 2_500;
pub const CART_PARTNER_NEAR_UNITS: f32 = 10.0;
pub const CART_RIDING_LOG_INTERVAL_MS: u64 = 2_000;
pub const CART_REFUSED_NOTICE_INTERVAL_MS: u64 = 3_000;
pub const GUEST_CART_REFUSED_TEXT: &str = "Only the host starts a cart ride";
