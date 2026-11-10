mod choice;

pub use choice::{CharacterChoice, character_from_launcher, choose_character};

pub const CHARACTER_ENV: &str = "NIER_COOP_CHARACTER";
pub const ROLE_ENV: &str = "NIER_COOP_ROLE";
pub const HOST_ROLE_TEXT: &str = "host";
pub const GUEST_ROLE_TEXT: &str = "guest";
