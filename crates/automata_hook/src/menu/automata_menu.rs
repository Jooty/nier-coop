use crate::net::link_not_started_status;
use automata_game::exe::ExeVariant;
use automata_game::save_redirect::SaveRedirectStatus;
use nc_hook::menu::{
    AreaChangeRefusal, AreaEntry, CoOpStatus, ExtraPanel, FixEntry, FixOutcome, GameMenu,
    StoryJumpEntry,
};

const NO_AREAS_YET_TEXT: &str = "no areas yet";
const NO_STORY_JUMPS_YET_TEXT: &str = "no story jumps yet";
const NO_FIXES_TEXT: &str = "no fixes";
const NO_START_STATUS_TEXT: &str = "start: not started";
const NO_PARTNER_YET_TEXT: &str = "no partner yet";

#[derive(Debug, Clone, Copy)]
pub struct AutomataMenu {
    variant: Option<ExeVariant>,
    build_commit: &'static str,
    save_status: fn() -> SaveRedirectStatus,
    start_status: fn() -> String,
    co_op: fn() -> CoOpStatus,
}

impl AutomataMenu {
    pub fn new(variant: Option<ExeVariant>, build_commit: &'static str) -> Self {
        Self {
            variant,
            build_commit,
            save_status: || SaveRedirectStatus::NotRedirected,
            start_status: || NO_START_STATUS_TEXT.to_owned(),
            co_op: link_not_started_status,
        }
    }

    pub fn with_save_status(mut self, save_status: fn() -> SaveRedirectStatus) -> Self {
        self.save_status = save_status;
        self
    }

    pub fn with_start_status(mut self, start_status: fn() -> String) -> Self {
        self.start_status = start_status;
        self
    }

    pub fn with_co_op_status(mut self, co_op: fn() -> CoOpStatus) -> Self {
        self.co_op = co_op;
        self
    }
}

impl GameMenu for AutomataMenu {
    fn co_op_status(&self) -> CoOpStatus {
        (self.co_op)()
    }

    fn areas(&self) -> Vec<AreaEntry> {
        Vec::new()
    }

    fn current_area(&self) -> Option<AreaEntry> {
        None
    }

    fn area_change_blocker(&self) -> Option<AreaChangeRefusal> {
        None
    }

    fn change_area(&self, _area: &AreaEntry) -> Result<(), String> {
        Err(NO_AREAS_YET_TEXT.to_owned())
    }

    fn go_to_partner(&self) -> Result<String, String> {
        Err(NO_PARTNER_YET_TEXT.to_owned())
    }

    fn bring_partner(&self) -> Result<String, String> {
        Err(NO_PARTNER_YET_TEXT.to_owned())
    }

    fn story_jumps(&self) -> Vec<StoryJumpEntry> {
        Vec::new()
    }

    fn run_story_jump(&self, _id: u32) -> Result<String, String> {
        Err(NO_STORY_JUMPS_YET_TEXT.to_owned())
    }

    fn fixes(&self) -> Vec<FixEntry> {
        Vec::new()
    }

    fn run_fix(&self, _id: u32) -> FixOutcome {
        FixOutcome::Refused(NO_FIXES_TEXT.to_owned())
    }

    fn status_lines(&self) -> Vec<String> {
        let variant = self.variant.map_or("unknown", ExeVariant::label);
        vec![
            format!("exe variant: {variant}"),
            format!("build: {}", self.build_commit),
            format!("link: {}", (self.co_op)().link),
            (self.save_status)().status_line().to_owned(),
            (self.start_status)(),
        ]
    }

    fn extra_panels(&self) -> Vec<ExtraPanel> {
        Vec::new()
    }
}
