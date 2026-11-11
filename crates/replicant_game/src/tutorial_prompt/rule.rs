#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptView {
    pub in_cutscene: bool,
    pub fade_active: bool,
    pub ui_dialog_state: i32,
    pub pause_menu_open: bool,
}

pub fn may_open_now(view: PromptView) -> bool {
    !view.in_cutscene && !view.fade_active && view.ui_dialog_state == 0 && !view.pause_menu_open
}
