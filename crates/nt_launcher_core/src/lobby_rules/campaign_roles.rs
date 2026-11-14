use crate::profile::{Character, GameProfile};

/// Who a player is in the co-op campaign: the host plays the story's hero, the guest the companion.
pub fn campaign_character(profile: &GameProfile, is_guest: bool) -> Option<&'static Character> {
    profile.characters.get(usize::from(is_guest))
}
