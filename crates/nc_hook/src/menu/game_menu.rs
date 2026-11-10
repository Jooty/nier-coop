use crate::menu::{
    AreaChangeRefusal, AreaEntry, CoOpStatus, ExtraPanel, FixEntry, FixOutcome, MenuNote,
    SectionKind, StoryJumpEntry,
};

pub trait GameMenu {
    fn co_op_status(&self) -> CoOpStatus;

    fn areas(&self) -> Vec<AreaEntry>;

    fn current_area(&self) -> Option<AreaEntry>;

    fn area_change_blocker(&self) -> Option<AreaChangeRefusal>;

    fn change_area(&self, area: &AreaEntry) -> Result<(), String>;

    fn go_to_partner(&self) -> Result<String, String>;

    fn bring_partner(&self) -> Result<String, String>;

    fn story_jumps(&self) -> Vec<StoryJumpEntry>;

    fn run_story_jump(&self, id: u32) -> Result<String, String>;

    fn fixes(&self) -> Vec<FixEntry>;

    fn run_fix(&self, id: u32) -> FixOutcome;

    fn status_lines(&self) -> Vec<String>;

    fn extra_panels(&self) -> Vec<ExtraPanel>;

    fn section_notes(&self, _section: SectionKind) -> Vec<MenuNote> {
        Vec::new()
    }
}
