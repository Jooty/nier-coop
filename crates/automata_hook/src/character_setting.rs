mod choice;

pub use choice::{
    CharacterChoice, character_from_launcher, choose_character, choose_role, role_from_launcher,
};

pub const CHARACTER_ENV: &str = "NIERTOGETHER_CHARACTER";
pub const ROLE_ENV: &str = "NIERTOGETHER_ROLE";
pub const HOST_ROLE_TEXT: &str = "host";
pub const GUEST_ROLE_TEXT: &str = "guest";
