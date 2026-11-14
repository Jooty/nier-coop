use hudhook::RenderContext;
use hudhook::imgui::{Context, Io, Key, Ui};
use nt_hook::overlay::OverlayGame;
use replicant_game::Replicant;

use crate::build_gate::{GateError, VerifiedBuild};
use crate::game_hooks;
use crate::memory::InProcessMemory;
use crate::overlay::area_point_panel::draw_area_point_panel;
use crate::overlay::choice_tag::draw_choice_tag;
use crate::overlay::click_sources;
use crate::overlay::co_op_panel::CoOpPanel;
use crate::overlay::diagnostics_window::draw_diagnostics_window;
use crate::overlay::event_log::draw_event_log;
use crate::overlay::hud_assets;
use crate::overlay::input;
use crate::overlay::minimap_dot::draw_minimap_dot;
use crate::overlay::mouse_panel::MouseDiagnostics;
use crate::overlay::partner_marker::draw_partner_marker;
use crate::overlay::party_frame::draw_party_frame;
use crate::overlay::settings_section::draw_settings_section;
use crate::overlay::trainer_panel::TrainerPanel;
use crate::player_settings;

const CURSOR_LATCH_KEY: Key = Key::F2;

pub(super) const DIAGNOSTICS_PANEL_ID: u32 = 0;
pub(super) const CO_OP_PANEL_ID: u32 = 1;
pub(super) const TRAINER_PANEL_ID: u32 = 2;
pub(super) const AREA_POINT_PANEL_ID: u32 = 3;

pub(crate) struct ReplicantOverlay {
    pub(super) gate: Result<VerifiedBuild, GateError>,
    pub(super) game: Option<Replicant<InProcessMemory>>,
    pub(super) area_point: i32,
    mouse: MouseDiagnostics,
    drawn_frames: u64,
    trainer: TrainerPanel,
    co_op: CoOpPanel,
}

impl ReplicantOverlay {
    pub(crate) fn new(gate: &Result<VerifiedBuild, GateError>) -> Self {
        let game = gate
            .as_ref()
            .ok()
            .map(|build| Replicant::new(InProcessMemory::new(build)));
        Self {
            gate: *gate,
            game,
            area_point: 0,
            mouse: MouseDiagnostics::default(),
            drawn_frames: 0,
            trainer: TrainerPanel::default(),
            co_op: CoOpPanel,
        }
    }
}

impl OverlayGame for ReplicantOverlay {
    fn initialize(&mut self, _context: &mut Context, render: &mut dyn RenderContext) {
        hud_assets::load(render);
    }

    fn prepare_input(&mut self, io: &mut Io, menu_visible: bool) -> bool {
        let taking_mouse = menu_visible && input::cursor_mode_requested();
        game_hooks::set_cursor_mode(taking_mouse);
        game_hooks::set_keyboard_taken_by_overlay(menu_visible && io.want_capture_keyboard);
        let clicks = click_sources::sample();
        if menu_visible {
            input::feed_real_mouse(io, taking_mouse, &clicks);
        } else {
            input::hide_pointer(io);
        }
        let blocking_game_mouse = menu_visible && io.want_capture_mouse;
        game_hooks::set_game_mouse_blocked(blocking_game_mouse);
        self.mouse = input::read_diagnostics(io, taking_mouse, clicks);
        taking_mouse
    }

    fn observe_window_message(&self, message: u32, wparam: usize, lparam: isize) -> bool {
        click_sources::observe_message(message, wparam, lparam)
    }

    fn on_frame(&mut self, _ui: &Ui) {
        player_settings::take_from_launcher();
        game_hooks::log_runner_state();
    }

    fn on_visible_frame(&mut self, ui: &Ui) {
        if ui.is_key_pressed_no_repeat(CURSOR_LATCH_KEY) {
            input::toggle_cursor_latch();
        }
        self.drawn_frames += 1;
    }

    fn draw_hud(&mut self, ui: &Ui) {
        draw_partner_marker(ui, self.game.as_ref());
        draw_minimap_dot(ui, self.game.as_ref());
        draw_party_frame(ui);
        draw_choice_tag(ui);
        draw_event_log(ui);
    }

    fn draw_settings(&mut self, ui: &Ui) {
        draw_settings_section(ui);
    }

    fn draw_extra_panel(&mut self, ui: &Ui, panel_id: u32, open: &mut bool) {
        match panel_id {
            DIAGNOSTICS_PANEL_ID => {
                draw_diagnostics_window(ui, &self.gate, self.drawn_frames, &self.mouse, open);
            }
            CO_OP_PANEL_ID => self.co_op.draw(ui, open),
            TRAINER_PANEL_ID => self.trainer.draw(ui, &self.gate, self.game.as_ref(), open),
            AREA_POINT_PANEL_ID => draw_area_point_panel(ui, &mut self.area_point, open),
            _ => {}
        }
    }
}
