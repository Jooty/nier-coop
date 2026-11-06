use std::time::Instant;

use hudhook::imgui::{Context, Io, Key, Ui};
use hudhook::{ImguiRenderLoop, MessageFilter, RenderContext};
use rc_game::Replicant;

use crate::build_gate::{GateError, VerifiedBuild};
use crate::memory::InProcessMemory;
use crate::overlay::MIN_GAP_BETWEEN_DRAWN_FRAMES;
use crate::overlay::debug_panel::draw_debug_panel;
use crate::overlay::levels_panel::LevelsPanel;
use crate::overlay::style::apply_classic_black_style;
use crate::overlay::trainer_panel::TrainerPanel;

const TOGGLE_KEY: Key = Key::Insert;

pub(super) struct DebugOverlay {
    visible: bool,
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

    fn render(&mut self, ui: &mut Ui) {
        if ui.is_key_pressed_no_repeat(TOGGLE_KEY) {
            self.visible = !self.visible;
        }
        if !self.visible || !self.claim_frame() {
            return;
        }
        draw_debug_panel(ui, &self.gate, self.game.as_ref(), self.drawn_frames);
        self.levels.draw(ui, &self.gate, self.game.as_ref());
        self.trainer.draw(ui, &self.gate, self.game.as_ref());
    }

    fn message_filter(&self, io: &Io) -> MessageFilter {
        if !self.visible {
            return MessageFilter::empty();
        }
        let mut filter = MessageFilter::empty();
        if io.want_capture_mouse {
            filter |= MessageFilter::InputMouse;
        }
        if io.want_capture_keyboard {
            filter |= MessageFilter::InputKeyboard;
        }
        filter
    }
}
