use std::path::{Path, PathBuf};

use nt_wire::SessionRules;

use crate::lobby_rules::{current_mode, lobby_permissions};
use crate::profile::{FreeRoamMap, GameProfile};

const MAP_BANNERS_FOLDER_NAME: &str = "map-banners";
const MAP_BANNER_EXTENSION: &str = "png";

/// The map the session's rules name, for a mode that starts on a map; the game's first map when the
/// index is unknown.
pub fn current_map(profile: &GameProfile, rules: SessionRules) -> Option<&'static FreeRoamMap> {
    if current_mode(profile, rules).is_none_or(|mode| mode.is_campaign()) {
        return None;
    }
    let maps = profile.free_roam_maps;
    maps.get(usize::from(rules.map)).or_else(|| maps.first())
}

/// The rules with the map at `map_index`, or `None` when that map does not exist or the mode has no maps.
pub fn rules_with_map(
    profile: &GameProfile,
    rules: SessionRules,
    map_index: u8,
) -> Option<SessionRules> {
    current_map(profile, rules)?;
    profile
        .free_roam_maps
        .get(usize::from(map_index))
        .map(|_| SessionRules {
            map: map_index,
            ..rules
        })
}

pub fn map_pick_offered(profile: &GameProfile, rules: SessionRules, viewer_is_guest: bool) -> bool {
    lobby_permissions(viewer_is_guest).change_options && current_map(profile, rules).is_some()
}

pub fn map_banner_path(launcher_data: &Path, game_id: &str, map_id: &str) -> PathBuf {
    launcher_data
        .join(MAP_BANNERS_FOLDER_NAME)
        .join(game_id)
        .join(format!("{map_id}.{MAP_BANNER_EXTENSION}"))
}
