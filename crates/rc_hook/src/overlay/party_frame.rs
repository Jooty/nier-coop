use hudhook::imgui::{DrawListMut, TextureId, Ui};
use rc_game::downed::DownedState;
use rc_game::party_frame::{
    PartyFrameLayout, PartyStatus, gauge_quads, health_fraction, party_frame_layout, party_status,
};

use crate::downed::downed_status;
use crate::net::{self, NetShared};
use crate::overlay::hud_assets::{HudTextures, textures};

const TEXT_COLOR: [f32; 4] = [0.84, 0.81, 0.71, 1.0];
const STATUS_COLOR: [f32; 4] = [0.93, 0.74, 0.62, 1.0];
const TEXT_SHADOW_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.7];
const TEXT_SHADOW_OFFSET_PIXELS: f32 = 1.5;
const ICON_COLOR: [f32; 4] = [0.84, 0.81, 0.71, 1.0];
const ICON_SEGMENTS: u32 = 20;
const ICON_RING_THICKNESS_PIXELS: f32 = 1.5;
const ICON_DOT_FRACTION: f32 = 0.38;
const STATUS_GAP_PIXELS: f32 = 10.0;
const WHITE: [f32; 4] = [1.0; 4];
const FILL_TINT_DOWNED: [f32; 4] = [0.92, 0.26, 0.2, 1.0];
const FILL_TINT_OTHER_AREA: [f32; 4] = [0.55, 0.78, 0.85, 1.0];
const FILL_TINT_MUTED: [f32; 4] = [0.6, 0.72, 0.64, 1.0];
const FILL_TINT_DISCONNECTED: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
const TRACK_DIM_DISCONNECTED: [f32; 4] = [1.0, 1.0, 1.0, 0.5];
const PLAIN_TRACK_COLOR: [f32; 4] = [0.29, 0.30, 0.26, 1.0];
const PLAIN_FILL_COLOR: [f32; 4] = [0.30, 0.82, 0.55, 1.0];

pub(super) fn draw_party_frame(ui: &Ui) {
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    if !view.options.show_party_frame {
        return;
    }
    let Some(remote) = view.remote.as_ref() else {
        return;
    };
    let status = status_of(&view);
    let activity = remote.state.activity;
    let [width, height] = ui.io().display_size;
    let layout = party_frame_layout(width, height);
    let draw_list = ui.get_background_draw_list();
    let assets = textures();
    draw_icon(&draw_list, &layout);
    let label = status_label(status).or(activity.label().filter(|_| status == PartyStatus::Up));
    let label_width = label.map_or(0.0, |label| text_width(ui, assets, label, &layout));
    let gap = if label.is_some() {
        STATUS_GAP_PIXELS * layout.scale
    } else {
        0.0
    };
    let name = fit_name(assets, &remote.name, &layout, label_width + gap);
    draw_text(
        &draw_list,
        assets,
        &name,
        layout.name_origin,
        &layout,
        TEXT_COLOR,
    );
    if let Some(label) = label {
        let origin = [
            layout.status_right_edge - label_width,
            layout.name_origin[1],
        ];
        draw_text(&draw_list, assets, label, origin, &layout, STATUS_COLOR);
    }
    let fraction = health_fraction(remote.state.hp, remote.state.max_hp);
    draw_gauge(
        &draw_list,
        assets,
        &layout,
        fraction,
        status,
        activity.world_paused(),
    );
}

fn status_of(view: &NetShared) -> PartyStatus {
    let remote = view.remote.as_ref();
    party_status(
        remote.is_some(),
        remote.map_or(0, |remote| remote.age().as_millis() as u64),
        remote
            .zip(view.local.as_ref())
            .is_some_and(|(remote, local)| remote.state.phase == local.phase),
        downed_status().partner == DownedState::Downed,
    )
}

fn status_label(status: PartyStatus) -> Option<&'static str> {
    match status {
        PartyStatus::Up => None,
        PartyStatus::Downed => Some("downed"),
        PartyStatus::OtherArea => Some("in another area"),
        PartyStatus::Disconnected => Some("disconnected"),
    }
}

fn fill_tint(status: PartyStatus) -> [f32; 4] {
    match status {
        PartyStatus::Up => WHITE,
        PartyStatus::Downed => FILL_TINT_DOWNED,
        PartyStatus::OtherArea => FILL_TINT_OTHER_AREA,
        PartyStatus::Disconnected => FILL_TINT_DISCONNECTED,
    }
}

fn text_width(ui: &Ui, assets: Option<&HudTextures>, text: &str, layout: &PartyFrameLayout) -> f32 {
    match assets {
        Some(assets) => assets.font.text_width(text, layout.name_scale),
        None => ui.calc_text_size(text)[0],
    }
}

fn fit_name(
    assets: Option<&HudTextures>,
    name: &str,
    layout: &PartyFrameLayout,
    reserved_width: f32,
) -> String {
    match assets {
        Some(assets) => assets.font.fit_text(
            name,
            layout.name_scale,
            layout.name_max_width - reserved_width,
        ),
        None => name.to_owned(),
    }
}

fn draw_icon(draw_list: &DrawListMut<'_>, layout: &PartyFrameLayout) {
    draw_list
        .add_circle(layout.icon_center, layout.icon_radius, ICON_COLOR)
        .num_segments(ICON_SEGMENTS)
        .thickness(ICON_RING_THICKNESS_PIXELS * layout.scale)
        .build();
    draw_list
        .add_circle(
            layout.icon_center,
            layout.icon_radius * ICON_DOT_FRACTION,
            ICON_COLOR,
        )
        .num_segments(ICON_SEGMENTS)
        .filled(true)
        .build();
}

fn draw_text(
    draw_list: &DrawListMut<'_>,
    assets: Option<&HudTextures>,
    text: &str,
    origin: [f32; 2],
    layout: &PartyFrameLayout,
    color: [f32; 4],
) {
    let Some(assets) = assets else {
        draw_list.add_text(origin, color, text);
        return;
    };
    let shadow = TEXT_SHADOW_OFFSET_PIXELS * layout.scale;
    for (offset, tint) in [(shadow, TEXT_SHADOW_COLOR), (0.0, color)] {
        let quads = assets.font.layout_text(
            text,
            [origin[0] + offset, origin[1] + offset],
            layout.name_scale,
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

fn draw_gauge(
    draw_list: &DrawListMut<'_>,
    assets: Option<&HudTextures>,
    layout: &PartyFrameLayout,
    fraction: f32,
    status: PartyStatus,
    muted: bool,
) {
    let quads = gauge_quads(layout.gauge_origin, layout.gauge_scale, fraction);
    let Some(assets) = assets else {
        draw_list
            .add_rect(quads.track_min, quads.track_max, PLAIN_TRACK_COLOR)
            .filled(true)
            .build();
        draw_list
            .add_rect(quads.fill_min, quads.fill_max, PLAIN_FILL_COLOR)
            .filled(true)
            .build();
        return;
    };
    let track_color = if status == PartyStatus::Disconnected {
        TRACK_DIM_DISCONNECTED
    } else {
        WHITE
    };
    draw_sprite(
        draw_list,
        assets.gauge_track,
        quads.track_min,
        quads.track_max,
        track_color,
    );
    if fraction > 0.0 {
        let (fill_texture, fill_color) = match status {
            PartyStatus::Up if muted => (assets.gauge_fill_white, FILL_TINT_MUTED),
            PartyStatus::Up => (assets.gauge_fill_green, WHITE),
            other => (assets.gauge_fill_white, fill_tint(other)),
        };
        draw_list
            .add_image(fill_texture, quads.fill_min, quads.fill_max)
            .uv_max([quads.fill_uv_right, 1.0])
            .col(fill_color)
            .build();
    }
    draw_sprite(
        draw_list,
        assets.gauge_line,
        quads.line_min,
        quads.line_max,
        WHITE,
    );
}

fn draw_sprite(
    draw_list: &DrawListMut<'_>,
    texture: TextureId,
    min: [f32; 2],
    max: [f32; 2],
    color: [f32; 4],
) {
    draw_list.add_image(texture, min, max).col(color).build();
}
