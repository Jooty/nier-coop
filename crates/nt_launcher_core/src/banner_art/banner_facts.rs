use crate::profile::game_by_id;

/// A point in the art, as fractions of its width and height, that a crop keeps in frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArtFocus {
    pub x: f32,
    pub y: f32,
}

/// What a game's home banner shows besides the install facts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BannerFacts {
    pub tags: &'static [&'static str],
    pub focus: ArtFocus,
}

pub fn banner_facts(game_id: &str) -> BannerFacts {
    game_by_id(game_id).banner()
}
