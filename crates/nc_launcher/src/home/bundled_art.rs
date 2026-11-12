const BUNDLED_BANNERS: [(&str, &[u8]); 0] = [];

/// A banner image built into the launcher for this game, from `assets/art/<game>-banner.*`. None is bundled yet;
/// an entry here wins over the Steam library cache.
pub fn bundled_banner(game_id: &str) -> Option<&'static [u8]> {
    BUNDLED_BANNERS
        .iter()
        .find(|(id, _)| *id == game_id)
        .map(|(_, bytes)| *bytes)
}
