use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use automata_game::hud_look::{
    ArchiveAsset, CRILAYLA_MAGIC, GAME_DATA_FOLDER_NAME, HUD_FONT_TABLE_ASSET,
    HUD_FONT_TEXTURE_ASSET, HUD_LAYOUT_ASSET, HudFont, HudPalette, UI_ARCHIVE_FILE_NAME,
    build_hud_font, hud_palette, unpack_crilayla,
};
use hudhook::RenderContext;
use hudhook::imgui::{DrawListMut, TextureId, Ui};
use nc_hook::diagnostics::log_line;

const PALETTE_WITHOUT_THE_GAMES_FILES: HudPalette = HudPalette {
    light: [1.0, 1.0, 1.0],
    dark: [0.0, 0.0, 0.0],
    accent: [1.0, 0.35, 0.3],
};

struct GameFont {
    font: HudFont,
    texture: TextureId,
}

struct HudLook {
    font: Option<GameFont>,
    palette: HudPalette,
}

/// A text's width and where its capital letters start and end below its top, in pixels.
pub(super) struct TextBox {
    pub width: f32,
    pub ink_top: f32,
    pub ink_bottom: f32,
}

static HUD_LOOK: OnceLock<HudLook> = OnceLock::new();

/// Reads the HUD font and the HUD colours from the player's own game files, once.
pub(crate) fn load_look(render: &mut dyn RenderContext) {
    HUD_LOOK.get_or_init(|| {
        let archive = archive_path();
        let font = archive
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|archive| load_font(render, archive));
        let font = match font {
            Ok(loaded) => {
                log_line(&format!(
                    "hud look: game font loaded ({} glyphs, {}x{} atlas) from {UI_ARCHIVE_FILE_NAME}",
                    loaded.font.glyph_count(),
                    loaded.font.texture_width,
                    loaded.font.texture_height
                ));
                Some(loaded)
            }
            Err(reason) => {
                log_line(&format!(
                    "hud look: game font NOT loaded ({reason}), text uses the plain font"
                ));
                None
            }
        };
        let palette = archive
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|archive| read_asset(archive, &HUD_LAYOUT_ASSET))
            .and_then(|layouts| {
                hud_palette(&layouts).ok_or_else(|| "the HUD layouts hold no palette".to_owned())
            });
        let palette = match palette {
            Ok(palette) => {
                log_line(&format!(
                    "hud look: game colours read from the HUD layouts in {UI_ARCHIVE_FILE_NAME} (light {}, dark {}, warning {})",
                    hex(palette.light),
                    hex(palette.dark),
                    hex(palette.accent)
                ));
                palette
            }
            Err(reason) => {
                log_line(&format!(
                    "hud look: game colours NOT read ({reason}), the HUD is plain white on black"
                ));
                PALETTE_WITHOUT_THE_GAMES_FILES
            }
        };
        HudLook { font, palette }
    });
}

pub(super) fn palette() -> HudPalette {
    HUD_LOOK
        .get()
        .map_or(PALETTE_WITHOUT_THE_GAMES_FILES, |look| look.palette)
}

pub(super) fn measure(ui: &Ui, text: &str, cell_pixels: f32) -> TextBox {
    match game_font() {
        Some(loaded) => {
            let scale = cell_pixels / loaded.font.cell_height;
            TextBox {
                width: loaded.font.text_width(text, scale),
                ink_top: loaded.font.ink_top * scale,
                ink_bottom: loaded.font.ink_bottom * scale,
            }
        }
        None => {
            let [width, height] = ui.calc_text_size(text);
            TextBox {
                width,
                ink_top: 0.0,
                ink_bottom: height,
            }
        }
    }
}

pub(super) fn draw_text(
    draw_list: &DrawListMut<'_>,
    text: &str,
    at: [f32; 2],
    cell_pixels: f32,
    color: [f32; 4],
    shadow: ([f32; 4], f32),
) {
    let Some(loaded) = game_font() else {
        draw_list.add_text([at[0] + shadow.1, at[1] + shadow.1], shadow.0, text);
        draw_list.add_text(at, color, text);
        return;
    };
    let scale = cell_pixels / loaded.font.cell_height;
    for (offset, tint) in [(shadow.1, shadow.0), (0.0, color)] {
        for quad in loaded
            .font
            .layout_text(text, [at[0] + offset, at[1] + offset], scale)
        {
            draw_list
                .add_image(loaded.texture, quad.min, quad.max)
                .uv_min(quad.uv_min)
                .uv_max(quad.uv_max)
                .col(tint)
                .build();
        }
    }
}

fn game_font() -> Option<&'static GameFont> {
    HUD_LOOK.get().and_then(|look| look.font.as_ref())
}

fn load_font(render: &mut dyn RenderContext, archive: &Path) -> Result<GameFont, String> {
    let tables = read_asset(archive, &HUD_FONT_TABLE_ASSET)?;
    let textures = read_asset(archive, &HUD_FONT_TEXTURE_ASSET)?;
    let font = build_hud_font(&tables, &textures)?;
    let texture = render
        .load_texture(&font.coverage_rgba, font.texture_width, font.texture_height)
        .map_err(|error| format!("texture upload: {error}"))?;
    Ok(GameFont { font, texture })
}

fn archive_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|error| format!("exe path: {error}"))?;
    let folder = exe.parent().ok_or("exe has no folder")?;
    Ok(folder
        .join(GAME_DATA_FOLDER_NAME)
        .join(UI_ARCHIVE_FILE_NAME))
}

fn read_asset(archive: &Path, asset: &ArchiveAsset) -> Result<Vec<u8>, String> {
    let mut file =
        File::open(archive).map_err(|error| format!("open {}: {error}", archive.display()))?;
    file.seek(SeekFrom::Start(asset.archive_offset))
        .map_err(|error| format!("seek: {error}"))?;
    let mut stored = vec![0_u8; asset.stored_size];
    file.read_exact(&mut stored)
        .map_err(|error| format!("read: {error}"))?;
    let bytes = if stored.starts_with(CRILAYLA_MAGIC) {
        unpack_crilayla(&stored).ok_or("a packed file did not unpack")?
    } else {
        stored
    };
    if bytes.len() != asset.unpacked_size {
        return Err("a file's size differs from the archive index of the known build".to_owned());
    }
    Ok(bytes)
}

fn hex(color: [f32; 3]) -> String {
    let byte = |channel: f32| (channel * 255.0).round() as u8;
    format!(
        "#{:02X}{:02X}{:02X}",
        byte(color[0]),
        byte(color[1]),
        byte(color[2])
    )
}
