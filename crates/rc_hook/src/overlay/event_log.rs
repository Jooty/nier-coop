use hudhook::imgui::Ui;
use rc_game::event_log::{EventLogLayout, event_log_layout};

use crate::event_log::{event_log_enabled, take_visible_events};
use crate::overlay::hud_assets::{HudTextures, textures};

const TEXT_COLOR: [f32; 3] = [0.88, 0.85, 0.75];
const TEXT_SHADOW_COLOR: [f32; 3] = [0.0, 0.0, 0.0];
const TEXT_SHADOW_ALPHA: f32 = 0.7;
const TEXT_SHADOW_OFFSET_PIXELS: f32 = 1.5;
const BACKDROP_ALPHA: f32 = 0.42;
const BACKDROP_SOFT_EDGE_ALPHA: f32 = 0.16;
const BACKDROP_SOFT_EDGE_PIXELS: f32 = 6.0;
const BACKDROP_ROUNDING_PIXELS: f32 = 4.0;
const FONT_LINE_FRACTION: f32 = 0.75;

pub(super) fn draw_event_log(ui: &Ui) {
    if !event_log_enabled() {
        return;
    }
    let events = take_visible_events();
    if events.is_empty() {
        return;
    }
    let [_, height] = ui.io().display_size;
    let layout = event_log_layout(height);
    let assets = textures();
    let draw_list = ui.get_background_draw_list();
    for (index, event) in events.iter().enumerate() {
        let from_bottom = events.len() - 1 - index;
        let top = layout.line_top(from_bottom);
        let width = text_width(ui, assets, &event.text, &layout);
        let min = [layout.left - layout.padding, top];
        let max = [
            layout.left + width + layout.padding,
            top + layout.line_height,
        ];
        let soft = BACKDROP_SOFT_EDGE_PIXELS * layout.scale;
        let rounding = BACKDROP_ROUNDING_PIXELS * layout.scale;
        draw_list
            .add_rect(
                [min[0] - soft, min[1] - soft],
                [max[0] + soft, max[1] + soft],
                [0.0, 0.0, 0.0, BACKDROP_SOFT_EDGE_ALPHA * event.alpha],
            )
            .filled(true)
            .rounding(rounding + soft)
            .build();
        draw_list
            .add_rect(min, max, [0.0, 0.0, 0.0, BACKDROP_ALPHA * event.alpha])
            .filled(true)
            .rounding(rounding)
            .build();
        let origin = [
            layout.left,
            top + (layout.line_height - FONT_LINE_FRACTION * layout.line_height) * 0.5,
        ];
        let shadow = TEXT_SHADOW_OFFSET_PIXELS * layout.scale;
        let shadow_color = with_alpha(TEXT_SHADOW_COLOR, TEXT_SHADOW_ALPHA * event.alpha);
        let color = with_alpha(TEXT_COLOR, event.alpha);
        let Some(assets) = assets else {
            draw_list.add_text(origin, color, &event.text);
            continue;
        };
        for (offset, tint) in [(shadow, shadow_color), (0.0, color)] {
            let quads = assets.font.layout_text(
                &event.text,
                [origin[0] + offset, origin[1] + offset],
                layout.text_scale,
            );
            for quad in quads {
                draw_list
                    .add_image(assets.font_texture, quad.min, quad.max)
                    .uv_min(quad.uv_min)
                    .uv_max(quad.uv_max)
                    .col(tint)
                    .build();
            }
        }
    }
}

fn with_alpha(color: [f32; 3], alpha: f32) -> [f32; 4] {
    [color[0], color[1], color[2], alpha]
}

fn text_width(ui: &Ui, assets: Option<&HudTextures>, text: &str, layout: &EventLogLayout) -> f32 {
    match assets {
        Some(assets) => assets.font.text_width(text, layout.text_scale),
        None => ui.calc_text_size(text)[0],
    }
}
