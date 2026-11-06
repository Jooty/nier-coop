use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use hudhook::imgui::{Context, Io, Key, Ui};
use hudhook::windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use hudhook::{BeforeWndProc, ImguiRenderLoop, MessageFilter, RenderContext};
use rc_game::Replicant;

use crate::build_gate::{GateError, VerifiedBuild};
use crate::game_hooks;
use crate::memory::InProcessMemory;
use crate::overlay::MIN_GAP_BETWEEN_DRAWN_FRAMES;
use crate::overlay::debug_panel::{SubmenuOpen, draw_debug_panel};
use crate::overlay::input;
use crate::overlay::levels_panel::LevelsPanel;
use crate::overlay::mouse_panel::MouseDiagnostics;
use crate::overlay::style::apply_classic_black_style;
use crate::overlay::trainer_panel::TrainerPanel;

const TOGGLE_KEY: Key = Key::Insert;

pub(super) struct DebugOverlay {
    visible: bool,
    cursor_mode: AtomicBool,
    submenus: SubmenuOpen,
    mouse: MouseDiagnostics,
    drawn_frames: u64,
    last_drawn_at: Option<Instant>,
    gate: Result<VerifiedBuild, GateError>,
    game: Option<Replicant<InProcessMemory>>,
    levels: LevelsPanel,
    trainer: TrainerPanel,
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
            last_drawn_at: None,
            gate,
            game,
            levels: LevelsPanel::default(),
            trainer: TrainerPanel::default(),
        }
    }

    fn claim_frame(&mut self) -> bool {
        let now = Instant::now();
        if let Some(last) = self.last_drawn_at
            && now.duration_since(last) < MIN_GAP_BETWEEN_DRAWN_FRAMES
        {
            return false;
        }
        self.last_drawn_at = Some(now);
        self.drawn_frames += 1;
        true
    }
}

impl ImguiRenderLoop for DebugOverlay {
    fn initialize<'a>(&'a mut self, context: &mut Context, _render: &'a mut dyn RenderContext) {
        apply_classic_black_style(context);
    }

    fn before_render<'a>(&'a mut self, context: &mut Context, _render: &'a mut dyn RenderContext) {
        let taking_mouse = self.visible && input::cursor_key_held();
        self.cursor_mode.store(taking_mouse, Ordering::Relaxed);
        game_hooks::set_cursor_mode(taking_mouse);
        game_hooks::set_keyboard_taken_by_overlay(
            self.visible && context.io().want_capture_keyboard,
        );
        if self.visible {
            input::feed_real_mouse(context.io_mut(), taking_mouse);
        } else {
            input::hide_pointer(context.io_mut());
        }
        let blocking_game_mouse = self.visible && context.io().want_capture_mouse;
        game_hooks::set_game_mouse_blocked(blocking_game_mouse);
        self.mouse = input::read_diagnostics(context.io(), taking_mouse);
    }

    fn render(&mut self, ui: &mut Ui) {
        if ui.is_key_pressed_no_repeat(TOGGLE_KEY) {
            self.visible = !self.visible;
        }
        if !self.visible || !self.claim_frame() {
            return;
        }
        draw_debug_panel(
            ui,
            &self.gate,
            self.game.as_ref(),
            self.drawn_frames,
            &self.mouse,
            &mut self.submenus,
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
    }

    fn before_wnd_proc(
        &self,
        hwnd: HWND,
        _umsg: u32,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> BeforeWndProc {
        input::remember_game_window(hwnd.0 as usize);
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
