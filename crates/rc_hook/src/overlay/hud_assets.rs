use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use hudhook::RenderContext;
use hudhook::imgui::{DrawListMut, TextureId, Ui};
use rc_game::party_frame::{
    ArcAsset, BATTLE_UI_ASSET, FONT_CELL_UNITS, GAME_DATA_FOLDER_NAME, GaugeSprites,
    HUD_FONT_EN_ASSET, HudFont, INIT_ARC_FILE_NAME, SpriteImage, parse_font, parse_gauge_sprites,
};
use ruzstd::decoding::StreamingDecoder;

use crate::diagnostics::log_line;

pub(super) struct HudTextures {
    pub font: HudFont,
    pub font_texture: TextureId,
}

pub(super) struct GaugeTextures {
    pub life_base: TextureId,
    pub life_bar: TextureId,
    pub magic_bar: TextureId,
}

static HUD_TEXTURES: OnceLock<Option<HudTextures>> = OnceLock::new();
static GAUGE_TEXTURES: OnceLock<Option<GaugeTextures>> = OnceLock::new();

pub(super) fn load(render: &mut dyn RenderContext) {
    let archive = init_archive_path();
    HUD_TEXTURES.get_or_init(|| {
        let loaded = archive
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|archive| build_font(render, archive));
        match loaded {
            Ok(textures) => {
                log_line(&format!(
                    "hud assets: game font loaded ({} glyphs, {}x{} atlas) from init.arc",
                    textures.font.glyph_count(),
                    textures.font.texture_width,
                    textures.font.texture_height
                ));
                Some(textures)
            }
            Err(reason) => {
                log_line(&format!(
                    "hud assets: game font NOT loaded ({reason}), text uses the plain font"
                ));
                None
            }
        }
    });
    GAUGE_TEXTURES.get_or_init(|| {
        let loaded = archive
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|archive| build_gauges(render, archive));
        match loaded {
            Ok(textures) => Some(textures),
            Err(reason) => {
                log_line(&format!(
                    "hud assets: gauge sprites NOT loaded ({reason}), bars use plain rectangles"
                ));
                None
            }
        }
    });
}

pub(super) fn textures() -> Option<&'static HudTextures> {
    HUD_TEXTURES.get().and_then(Option::as_ref)
}

pub(super) fn gauge_textures() -> Option<&'static GaugeTextures> {
    GAUGE_TEXTURES.get().and_then(Option::as_ref)
}

pub(super) fn text_size(ui: &Ui, text: &str, size_px: f32) -> [f32; 2] {
    match textures() {
        Some(assets) => [
            assets.font.text_width(text, size_px / FONT_CELL_UNITS),
            size_px,
        ],
        None => ui.calc_text_size(text),
    }
}

pub(super) fn draw_text(
    draw_list: &DrawListMut<'_>,
    text: &str,
    at: [f32; 2],
    size_px: f32,
    color: [f32; 4],
    shadow: ([f32; 4], f32),
) {
    let Some(assets) = textures() else {
        draw_list.add_text([at[0] + shadow.1, at[1] + shadow.1], shadow.0, text);
        draw_list.add_text(at, color, text);
        return;
    };
    let scale = size_px / FONT_CELL_UNITS;
    for (offset, tint) in [(shadow.1, shadow.0), (0.0, color)] {
        for quad in assets
            .font
            .layout_text(text, [at[0] + offset, at[1] + offset], scale)
        {
            draw_list
                .add_image(assets.font_texture, quad.min, quad.max)
                .uv_min(quad.uv_min)
                .uv_max(quad.uv_max)
                .col(tint)
                .build();
        }
    }
}

fn build_font(render: &mut dyn RenderContext, archive: &Path) -> Result<HudTextures, String> {
    let font_pack = read_archive_asset(archive, &HUD_FONT_EN_ASSET)?;
    let font = parse_font(&font_pack).ok_or("font pack did not parse")?;
    let font_texture = upload(
        render,
        &font.coverage_rgba,
        font.texture_width,
        font.texture_height,
    )?;
    Ok(HudTextures { font, font_texture })
}

fn build_gauges(render: &mut dyn RenderContext, archive: &Path) -> Result<GaugeTextures, String> {
    let battle_pack = read_archive_asset(archive, &BATTLE_UI_ASSET)?;
    let GaugeSprites {
        life_base,
        magic_base,
        life_bar,
        magic_bar,
    } = parse_gauge_sprites(&battle_pack).ok_or("battle atlas did not parse")?;
    let sizes = [
        ("base_hp", &life_base),
        ("base_mp", &magic_base),
        ("player_bar", &life_bar),
        ("magic_bar", &magic_bar),
    ]
    .iter()
    .map(|(name, sprite)| format!("{name} {}x{}", sprite.width, sprite.height))
    .collect::<Vec<_>>()
    .join(", ");
    let textures = GaugeTextures {
        life_base: upload_sprite(render, &life_base)?,
        life_bar: upload_sprite(render, &life_bar)?,
        magic_bar: upload_sprite(render, &magic_bar)?,
    };
    log_line(&format!(
        "hud assets: gauge sprites loaded from battle_0 in init.arc ({sizes})"
    ));
    Ok(textures)
}

fn upload_sprite(
    render: &mut dyn RenderContext,
    sprite: &SpriteImage,
) -> Result<TextureId, String> {
    upload(render, &sprite.rgba, sprite.width, sprite.height)
}

fn upload(
    render: &mut dyn RenderContext,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<TextureId, String> {
    render
        .load_texture(rgba, width, height)
        .map_err(|error| format!("texture upload: {error}"))
}

fn init_archive_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|error| format!("exe path: {error}"))?;
    let folder = exe.parent().ok_or("exe has no folder")?;
    Ok(folder.join(GAME_DATA_FOLDER_NAME).join(INIT_ARC_FILE_NAME))
}

fn read_archive_asset(archive: &Path, asset: &ArcAsset) -> Result<Vec<u8>, String> {
    let mut file =
        File::open(archive).map_err(|error| format!("open {}: {error}", archive.display()))?;
    file.seek(SeekFrom::Start(asset.archive_offset))
        .map_err(|error| format!("seek: {error}"))?;
    let mut compressed = vec![0u8; asset.compressed_size];
    file.read_exact(&mut compressed)
        .map_err(|error| format!("read: {error}"))?;
    let mut decoder =
        StreamingDecoder::new(compressed.as_slice()).map_err(|error| format!("zstd: {error}"))?;
    let mut decoded = Vec::with_capacity(asset.decoded_size);
    decoder
        .read_to_end(&mut decoded)
        .map_err(|error| format!("zstd: {error}"))?;
    if decoded.len() != asset.decoded_size {
        return Err("decoded size differs from the archive index".to_owned());
    }
    Ok(decoded)
}
