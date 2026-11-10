use automata_game::exe::ExeVariant;
use nc_hook::menu::{
    AreaChangeRefusal, AreaEntry, CoOpStatus, ExtraPanel, FixEntry, FixOutcome, GameMenu,
    StoryJumpEntry,
};

const LINK_NOT_STARTED_TEXT: &str = "not connected (the link is not built yet)";
const NO_AREAS_YET_TEXT: &str = "no areas yet";
const NO_STORY_JUMPS_YET_TEXT: &str = "no story jumps yet";
const NO_FIXES_TEXT: &str = "no fixes";
const NO_PARTNER_YET_TEXT: &str = "no partner yet";

#[derive(Debug, Clone, Copy)]
pub struct AutomataMenu {
    variant: Option<ExeVariant>,
    build_commit: &'static str,
}

impl AutomataMenu {
    pub fn new(variant: Option<ExeVariant>, build_commit: &'static str) -> Self {
        Self {
            variant,
            build_commit,
        }
    }
}

impl GameMenu for AutomataMenu {
    fn co_op_status(&self) -> CoOpStatus {
        CoOpStatus {
            link: LINK_NOT_STARTED_TEXT.to_owned(),
            players: Vec::new(),
            ping_ms: None,
        }
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
            format!("link: {LINK_NOT_STARTED_TEXT}"),
        ]
    }

    fn extra_panels(&self) -> Vec<ExtraPanel> {
        Vec::new()
    }
}
