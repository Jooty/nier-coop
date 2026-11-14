use hudhook::RenderContext;
use hudhook::imgui::{Context, Io, Ui};

use crate::menu::GameMenu;

pub trait OverlayGame: GameMenu + Send + Sync + 'static {
    fn initialize(&mut self, context: &mut Context, render: &mut dyn RenderContext);

    fn prepare_input(&mut self, io: &mut Io, menu_visible: bool) -> bool;

    fn observe_window_message(&self, message: u32, wparam: usize, lparam: isize) -> bool;

    fn on_frame(&mut self, ui: &Ui);

    fn on_visible_frame(&mut self, ui: &Ui);

    fn draw_hud(&mut self, ui: &Ui);

    fn draw_settings(&mut self, ui: &Ui);

    fn draw_extra_panel(&mut self, ui: &Ui, panel_id: u32, open: &mut bool);
}
