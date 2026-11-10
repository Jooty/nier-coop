use hudhook::imgui::{Condition, Ui};
use replicant_game::offsets::MAX_PHASE_POINT;

pub(super) fn draw_area_point_panel(ui: &Ui, point: &mut i32, open: &mut bool) {
    if !*open {
        return;
    }
    ui.window("area point")
        .opened(open)
        .position([372.0, 16.0], Condition::FirstUseEver)
        .size([300.0, 90.0], Condition::FirstUseEver)
        .build(|| {
            ui.input_int("point", point).build();
            *point = (*point).clamp(0, MAX_PHASE_POINT);
            ui.text_wrapped("the spawn point the map teleporter asks the game for");
        });
}
