mod choice;
mod lines;
mod partner_fill;
mod roll;
mod session_partner;

pub use choice::LoadingChoice;
pub use lines::LoadingText;
pub use partner_fill::{clean_partner_name, fill_partner};
pub use session_partner::SessionPartner;

pub const PARTNER_PLACEHOLDER: &str = "{partner}";
pub const MAX_TEXT_BYTES: usize = 4095;
pub const MAX_PARTNER_NAME_CHARS: usize = 31;
pub const SHOW_ONE_LOADING_SCREEN_IN: u64 = 3;
