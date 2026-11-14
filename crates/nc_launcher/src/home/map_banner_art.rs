use std::collections::HashMap;
use std::path::PathBuf;

use eframe::egui::{Context, TextureHandle, TextureOptions};
use nc_launcher_core::map_banner_path;

use crate::config::data_dir;
use crate::home::banner_art::decode_banner_image;

const MAP_BANNER_TEXTURE_NAME_PREFIX: &str = "map-banner-";

/// Each free roam map's banner, read from the launcher's data folder (then beside the launcher) the first
/// time a lobby shows it; a map with no banner file has none.
pub struct MapBannerStore {
    roots: Vec<PathBuf>,
    textures: HashMap<(&'static str, &'static str), Option<TextureHandle>>,
}

impl Default for MapBannerStore {
    fn default() -> Self {
        let beside_the_launcher = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(PathBuf::from));
        Self::with_roots(data_dir().into_iter().chain(beside_the_launcher).collect())
    }
}

impl MapBannerStore {
    pub fn with_roots(roots: Vec<PathBuf>) -> Self {
        Self {
            roots,
            textures: HashMap::new(),
        }
    }

    pub fn texture(
        &mut self,
        ctx: &Context,
        game_id: &'static str,
        map_id: &'static str,
    ) -> Option<TextureHandle> {
        let roots = &self.roots;
        self.textures
            .entry((game_id, map_id))
            .or_insert_with(|| {
                let image = roots.iter().find_map(|root| {
                    let bytes = std::fs::read(map_banner_path(root, game_id, map_id)).ok()?;
                    decode_banner_image(&bytes)
                })?;
                let name = format!("{MAP_BANNER_TEXTURE_NAME_PREFIX}{game_id}-{map_id}");
                Some(ctx.load_texture(name, image, TextureOptions::LINEAR))
            })
            .clone()
    }
}
