use std::sync::{Mutex, PoisonError};

use automata_game::co_op_log::{
    ManyCoOpLines, OtherSeen, OverlayLog, PlayerSeen, downed_effect_line, overlay_log_layout,
};
use automata_game::downed::DownedEffect;
use automata_game::partner_hud::shown_name;
use automata_proto::PlayerState;
use hudhook::imgui::Ui;
use nt_hook::diagnostics::log_line;
use nt_hook::net::{NetState, Player, net_clock_ms};

use crate::hud::assets;
use crate::hud::game_log_hook::post_to_game_log;
use crate::hud::settings::{co_op_lines_shown, game_log_used};
use crate::hud::{
    OVERLAY_LOG_BACKDROP_ALPHA, OVERLAY_LOG_BACKDROP_ROUNDING_REFERENCE_PIXELS, TEXT_SHADOW_ALPHA,
    TEXT_SHADOW_OFFSET_REFERENCE_PIXELS,
};
use crate::net::{self, NetShared, PartnerWatch, local_in_play};

static CO_OP_LINES: Mutex<ManyCoOpLines> = Mutex::new(ManyCoOpLines::new());
static OVERLAY_LOG: Mutex<OverlayLog> = Mutex::new(OverlayLog::new());

/// Game thread, once a frame: tells the player what changed in the session since the last frame.
pub(super) fn note_session(shared: &NetShared, players: &[&Player<PlayerState, PartnerWatch>]) {
    let names: Vec<String> = players
        .iter()
        .map(|player| shown_name(&player.name))
        .collect();
    let others: Vec<OtherSeen<'_>> = players
        .iter()
        .zip(&names)
        .map(|(player, name)| OtherSeen {
            slot: player.slot,
            name,
            state: player.latest.as_ref().map(|remote| PlayerSeen {
                phase_id: remote.state.phase_id,
                health: remote.state.health,
            }),
        })
        .collect();
    let local = shared
        .local
        .as_ref()
        .filter(|_| local_in_play())
        .map(|local| PlayerSeen {
            phase_id: local.phase_id,
            health: local.health,
        });
    let lines = CO_OP_LINES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note(
            shared.state == NetState::Connected,
            local,
            &others,
            net_clock_ms(),
        );
    for line in lines {
        post_line(&line);
    }
}

pub(crate) fn post_downed_effect_line(effect: DownedEffect) {
    let Some(net) = net::handle() else {
        return;
    };
    let name = shown_name(net.view().partner_name().unwrap_or_default());
    if let Some(line) = downed_effect_line(effect, &name) {
        post_line(&line);
    }
}

pub(crate) fn draw_overlay_log(ui: &Ui) {
    let lines = OVERLAY_LOG
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .visible(net_clock_ms());
    if lines.is_empty() || !co_op_lines_shown() {
        return;
    }
    let [_, height] = ui.io().display_size;
    let layout = overlay_log_layout(height);
    let palette = assets::palette();
    let draw_list = ui.get_background_draw_list();
    for (index, line) in lines.iter().enumerate() {
        let top = layout.line_top(lines.len() - 1 - index);
        let text = assets::measure(ui, &line.text, layout.text_cell);
        draw_list
            .add_rect(
                [layout.left - layout.padding, top],
                [
                    layout.left + text.width + layout.padding,
                    top + layout.line_height,
                ],
                [
                    palette.dark[0],
                    palette.dark[1],
                    palette.dark[2],
                    OVERLAY_LOG_BACKDROP_ALPHA * line.alpha,
                ],
            )
            .filled(true)
            .rounding(OVERLAY_LOG_BACKDROP_ROUNDING_REFERENCE_PIXELS * layout.scale)
            .build();
        let ink_height = text.ink_bottom - text.ink_top;
        assets::draw_text(
            &draw_list,
            &line.text,
            [
                layout.left,
                top + (layout.line_height - ink_height) * 0.5 - text.ink_top,
            ],
            layout.text_cell,
            [
                palette.light[0],
                palette.light[1],
                palette.light[2],
                line.alpha,
            ],
            (
                [0.0, 0.0, 0.0, TEXT_SHADOW_ALPHA * line.alpha],
                TEXT_SHADOW_OFFSET_REFERENCE_PIXELS * layout.scale,
            ),
        );
    }
}

/// Game thread only: one line in the game's own log, or drawn by the overlay.
pub(crate) fn post_line(line: &str) {
    if !co_op_lines_shown() {
        log_line(&format!(
            "co-op log: not shown, lines are turned off: {line}"
        ));
        return;
    }
    if game_log_used() && post_to_game_log(line) {
        log_line(&format!("co-op log: given to the game's own log: {line}"));
        return;
    }
    let posted = OVERLAY_LOG
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .post(line, net_clock_ms());
    if posted {
        log_line(&format!("co-op log: drawn by the overlay: {line}"));
    }
}
