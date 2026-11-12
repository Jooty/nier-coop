use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};
use image::imageops::FilterType;
use nc_launcher_core::{BannerArtSource, find_banner_art};

use crate::home::GameCard;
use crate::home::bundled_art::bundled_banner;
use crate::steam::steam_roots;

const MAX_ART_WIDTH_PX: u32 = 1920;
const ART_TEXTURE_NAME_PREFIX: &str = "banner-art-";

/// What a banner can draw this frame.
#[derive(Clone)]
pub enum BannerArtState {
    Pending,
    Missing,
    Ready(TextureHandle),
}

enum ArtSlot {
    Decoding(Receiver<Option<ColorImage>>),
    Decoded(ColorImage),
    Uploaded(TextureHandle),
    Missing,
}

/// Each game's banner art, looked up and decoded once on a thread of its own, then kept as a texture.
#[derive(Default)]
pub struct BannerArtStore {
    slots: HashMap<&'static str, ArtSlot>,
}

impl BannerArtStore {
    pub fn load_new_cards(&mut self, cards: &[GameCard], ctx: &Context) {
        for card in cards {
            let game_id = card.profile.game_id;
            if self.slots.contains_key(game_id) {
                continue;
            }
            let app_id = card.profile.steam_app_id;
            let library_root = card
                .install
                .as_ref()
                .and_then(|game| game.steamapps_dir.parent().map(PathBuf::from));
            let (sender, receiver) = channel();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let roots: Vec<PathBuf> = library_root.into_iter().chain(steam_roots()).collect();
                let image = match find_banner_art(bundled_banner(game_id), &roots, app_id) {
                    BannerArtSource::Bundled(bytes) => decode_banner_image(bytes),
                    BannerArtSource::File(path) => std::fs::read(path)
                        .ok()
                        .and_then(|bytes| decode_banner_image(&bytes)),
                    BannerArtSource::Shapes => None,
                };
                let _ = sender.send(image);
                ctx.request_repaint();
            });
            self.slots.insert(game_id, ArtSlot::Decoding(receiver));
        }
    }

    /// Sets a game's art directly, already decoded; `None` means the game has none.
    pub fn insert_image(&mut self, game_id: &'static str, image: Option<ColorImage>) {
        let slot = image.map_or(ArtSlot::Missing, ArtSlot::Decoded);
        self.slots.insert(game_id, slot);
    }

    pub fn state(&mut self, ctx: &Context, game_id: &str) -> BannerArtState {
        let Some(slot) = self.slots.get_mut(game_id) else {
            return BannerArtState::Pending;
        };
        if let ArtSlot::Decoding(receiver) = slot {
            match receiver.try_recv() {
                Ok(Some(image)) => *slot = ArtSlot::Decoded(image),
                Ok(None) | Err(TryRecvError::Disconnected) => *slot = ArtSlot::Missing,
                Err(TryRecvError::Empty) => return BannerArtState::Pending,
            }
        }
        if let ArtSlot::Decoded(image) = slot {
            let image = std::mem::take(image);
            let name = format!("{ART_TEXTURE_NAME_PREFIX}{game_id}");
            *slot = ArtSlot::Uploaded(ctx.load_texture(name, image, TextureOptions::LINEAR));
        }
        match slot {
            ArtSlot::Uploaded(texture) => BannerArtState::Ready(texture.clone()),
            _ => BannerArtState::Missing,
        }
    }
}

pub fn decode_banner_image(bytes: &[u8]) -> Option<ColorImage> {
    let mut image = image::load_from_memory(bytes).ok()?;
    if image.width() > MAX_ART_WIDTH_PX {
        image = image.resize(MAX_ART_WIDTH_PX, u32::MAX, FilterType::Triangle);
    }
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}
