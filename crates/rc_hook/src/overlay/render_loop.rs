use std::sync::atomic::{AtomicBool, Ordering};

use hudhook::imgui::{Context, Io, Key, Ui};
use hudhook::windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use hudhook::{BeforeWndProc, ImguiRenderLoop, MessageFilter, RenderContext};
use rc_game::Replicant;

use crate::build_gate::{GateError, VerifiedBuild};
use crate::game_hooks;
use crate::memory::InProcessMemory;
use crate::overlay::click_sources;
use crate::overlay::co_op_panel::CoOpPanel;
use crate::overlay::debug_panel::{SubmenuOpen, draw_debug_panel, draw_diagnostics_window};
use crate::overlay::event_log::draw_event_log;
use crate::overlay::hud_assets;
use crate::overlay::input;
use crate::overlay::levels_panel::LevelsPanel;
use crate::overlay::minimap_dot::draw_minimap_dot;
use crate::overlay::mouse_panel::MouseDiagnostics;
use crate::overlay::partner_marker::draw_partner_marker;
use crate::overlay::party_frame::draw_party_frame;
use crate::overlay::story_jump_panel::StoryJumpPanel;
use crate::overlay::style::apply_shared_style;
use crate::overlay::swapchain_window;
use crate::overlay::trainer_panel::TrainerPanel;
use crate::player_settings;

const TOGGLE_KEY: Key = Key::Insert;
const CURSOR_LATCH_KEY: Key = Key::F2;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_MOUSELAST: u32 = 0x0209;

pub(super) struct DebugOverlay {
    visible: bool,
    cursor_mode: AtomicBool,
    submenus: SubmenuOpen,
    mouse: MouseDiagnostics,
    drawn_frames: u64,
    gate: Result<VerifiedBuild, GateError>,
    game: Option<Replicant<InProcessMemory>>,
    levels: LevelsPanel,
    trainer: TrainerPanel,
    co_op: CoOpPanel,
    story_jump: StoryJumpPanel,
}

impl DebugOverlay {
    pub(super) fn new(gate: Result<VerifiedBuild, GateError>) -> Self {
        let game = gate
            .as_ref()
            .ok()
            .map(|build| Replicant::new(InProcessMemory::new(build)));
        Self {
            visible: false,
            cursor_mode: AtomicBool::new(false),
            submenus: SubmenuOpen::default(),
            mouse: MouseDiagnostics::default(),
            drawn_frames: 0,
            gate,
            game,
            levels: LevelsPanel::default(),
            trainer: TrainerPanel::default(),
            co_op: CoOpPanel,
            story_jump: StoryJumpPanel::default(),
        }
    }
}

impl ImguiRenderLoop for DebugOverlay {
    fn initialize<'a>(&'a mut self, context: &mut Context, render: &'a mut dyn RenderContext) {
        apply_shared_style(context);
        hud_assets::load(render);
    }

    fn before_render<'a>(&'a mut self, context: &mut Context, _render: &'a mut dyn RenderContext) {
        let taking_mouse = self.visible && input::cursor_mode_requested();
        self.cursor_mode.store(taking_mouse, Ordering::Relaxed);
        game_hooks::set_cursor_mode(taking_mouse);
        game_hooks::set_keyboard_taken_by_overlay(
            self.visible && context.io().want_capture_keyboard,
        );
        let clicks = click_sources::sample();
        if self.visible {
            input::feed_real_mouse(context.io_mut(), taking_mouse, &clicks);
        } else {
            input::hide_pointer(context.io_mut());
        }
        let blocking_game_mouse = self.visible && context.io().want_capture_mouse;
        game_hooks::set_game_mouse_blocked(blocking_game_mouse);
        self.mouse = input::read_diagnostics(context.io(), taking_mouse, clicks);
    }

    fn render(&mut self, ui: &mut Ui) {
        player_settings::take_from_launcher();
        game_hooks::log_runner_state();
        if ui.is_key_pressed_no_repeat(TOGGLE_KEY) {
            self.visible = !self.visible;
        }
        if self.visible && ui.is_key_pressed_no_repeat(CURSOR_LATCH_KEY) {
            input::toggle_cursor_latch();
        }
        draw_partner_marker(ui, self.game.as_ref());
        draw_minimap_dot(ui, self.game.as_ref());
        draw_party_frame(ui);
        draw_event_log(ui);
        if !self.visible {
            return;
        }
        // draw every ImGui frame: a frame without the windows leaves nothing hovered on the next
        // one, so a click landing in either frame is lost
        self.drawn_frames += 1;
        draw_debug_panel(ui, &self.gate, self.game.as_ref(), &mut self.submenus);
        draw_diagnostics_window(
            ui,
            &self.gate,
            self.drawn_frames,
            &self.mouse,
            &mut self.submenus.diagnostics,
        );
        self.levels.draw(
            ui,
            &self.gate,
            self.game.as_ref(),
            &mut self.submenus.levels,
        );
        self.trainer.draw(
            ui,
            &self.gate,
            self.game.as_ref(),
            &mut self.submenus.trainer,
        );
        self.co_op.draw(ui, &mut self.submenus.co_op);
        self.story_jump.draw(ui, &mut self.submenus.story_jump);
    }

    fn before_wnd_proc(
        &self,
        hwnd: HWND,
        umsg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> BeforeWndProc {
        swapchain_window::remember(hwnd.0 as usize);
        if (WM_MOUSEMOVE..=WM_MOUSELAST).contains(&umsg) {
            swapchain_window::remember_client_position(lparam.0);
        }
        if click_sources::observe_message(umsg, wparam.0, lparam.0) {
            return BeforeWndProc::Break;
        }
        BeforeWndProc::Continue
    }

    fn message_filter(&self, io: &Io) -> MessageFilter {
        if !self.visible {
            return MessageFilter::empty();
        }
        let mut filter = MessageFilter::empty();
        if self.cursor_mode.load(Ordering::Relaxed) {
            filter |= MessageFilter::InputMouse | MessageFilter::InputRaw;
        }
        if io.want_capture_mouse {
            filter |= MessageFilter::InputMouse;
        }
        if io.want_capture_keyboard {
            filter |= MessageFilter::InputKeyboard;
        }
        filter
    }
}
