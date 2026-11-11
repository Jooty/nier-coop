use hudhook::RenderContext;
use hudhook::imgui::{Context, Io, Ui};
use nc_hook::overlay::OverlayGame;

use crate::game_hooks::set_menu_open;
use crate::hud;
use crate::menu::AutomataMenu;

impl OverlayGame for AutomataMenu {
    fn initialize(&mut self, _context: &mut Context, render: &mut dyn RenderContext) {
        hud::load_look(render);
    }

    fn prepare_input(&mut self, _io: &mut Io, menu_visible: bool) -> bool {
        set_menu_open(menu_visible);
        false
    }

    fn observe_window_message(&self, _message: u32, _wparam: usize, _lparam: isize) -> bool {
        false
    }

    fn on_frame(&mut self, _ui: &Ui) {}

    fn on_visible_frame(&mut self, _ui: &Ui) {}

    fn draw_hud(&mut self, ui: &Ui) {
        hud::draw_name_tag(ui);
    }

    fn draw_settings(&mut self, ui: &Ui) {
        hud::draw_hud_settings(ui);
    }

    fn draw_extra_panel(&mut self, _ui: &Ui, _panel_id: u32, _open: &mut bool) {}
}
