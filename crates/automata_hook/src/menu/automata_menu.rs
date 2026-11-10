use automata_game::exe::ExeVariant;
use automata_game::save_redirect::SaveRedirectStatus;
use automata_game::travel::{GoToPartnerRefusal, JumpRefusal, TravelPoint, TravelView};
use nc_hook::menu::{
    AreaChangeRefusal, AreaEntry, CoOpStatus, ExtraPanel, FixEntry, FixOutcome, GameMenu, MenuNote,
    SectionKind, StoryJumpEntry,
};

use crate::menu::TeleportHooks;
use crate::net::link_not_started_status;

const NO_AREAS_YET_TEXT: &str = "no areas yet";
const NO_STORY_JUMPS_YET_TEXT: &str = "no story jumps yet";
const NO_FIXES_TEXT: &str = "no fixes";
const NO_START_STATUS_TEXT: &str = "start: not started";
const NO_PARTNER_YET_TEXT: &str = "no partner yet";
const NO_STORY_STATUS_TEXT: &str = "story: gate not installed";
const NO_PARTNER_BODY_STATUS_TEXT: &str = "partner body: no session";
const NO_TRAVEL_POINTS_YET_TEXT: &str =
    "no travel points yet: the list fills once the world is loaded";

#[derive(Debug, Clone, Copy)]
pub struct AutomataMenu {
    variant: Option<ExeVariant>,
    build_commit: &'static str,
    save_status: fn() -> SaveRedirectStatus,
    start_status: fn() -> String,
    story_status: fn() -> String,
    partner_body_status: fn() -> String,
    co_op: fn() -> CoOpStatus,
    teleport: Option<TeleportHooks>,
}

impl AutomataMenu {
    pub fn new(variant: Option<ExeVariant>, build_commit: &'static str) -> Self {
        Self {
            variant,
            build_commit,
            save_status: || SaveRedirectStatus::NotRedirected,
            start_status: || NO_START_STATUS_TEXT.to_owned(),
            story_status: || NO_STORY_STATUS_TEXT.to_owned(),
            partner_body_status: || NO_PARTNER_BODY_STATUS_TEXT.to_owned(),
            co_op: link_not_started_status,
            teleport: None,
        }
    }

    pub fn with_teleport(mut self, teleport: TeleportHooks) -> Self {
        self.teleport = Some(teleport);
        self
    }

    pub fn with_save_status(mut self, save_status: fn() -> SaveRedirectStatus) -> Self {
        self.save_status = save_status;
        self
    }

    pub fn with_start_status(mut self, start_status: fn() -> String) -> Self {
        self.start_status = start_status;
        self
    }

    pub fn with_story_status(mut self, story_status: fn() -> String) -> Self {
        self.story_status = story_status;
        self
    }

    pub fn with_partner_body_status(mut self, partner_body_status: fn() -> String) -> Self {
        self.partner_body_status = partner_body_status;
        self
    }

    pub fn with_co_op_status(mut self, co_op: fn() -> CoOpStatus) -> Self {
        self.co_op = co_op;
        self
    }

    fn travel_view(&self) -> Option<TravelView> {
        self.teleport.map(|teleport| (teleport.view)())
    }
}

impl GameMenu for AutomataMenu {
    fn co_op_status(&self) -> CoOpStatus {
        (self.co_op)()
    }

    fn areas(&self) -> Vec<AreaEntry> {
        self.travel_view()
            .map(|view| view.points().into_iter().map(area_of).collect())
            .unwrap_or_default()
    }

    fn current_area(&self) -> Option<AreaEntry> {
        self.travel_view()?.current.map(area_of)
    }

    fn area_change_blocker(&self) -> Option<AreaChangeRefusal> {
        self.travel_view()?.jump_blocker().map(refusal_of)
    }

    fn change_area(&self, area: &AreaEntry) -> Result<(), String> {
        let Some(teleport) = self.teleport else {
            return Err(NO_AREAS_YET_TEXT.to_owned());
        };
        let point = TravelPoint::from_index(area.id)
            .ok_or_else(|| format!("unknown travel point {}", area.id))?;
        if let Some(refusal) = (teleport.view)().point_blocker(point) {
            return Err(refusal.to_string());
        }
        (teleport.ask_jump)(point)
    }

    fn area_change_in_progress(&self) -> bool {
        self.travel_view().is_some_and(|view| view.jump_under_way)
    }

    fn no_areas_note(&self) -> Option<String> {
        self.teleport.map(|_| NO_TRAVEL_POINTS_YET_TEXT.to_owned())
    }

    fn go_to_partner(&self) -> Result<String, String> {
        let Some(teleport) = self.teleport else {
            return Err(NO_PARTNER_YET_TEXT.to_owned());
        };
        if !(teleport.view)().partner_body_present {
            return Err(format!("not moved: {}", GoToPartnerRefusal::NoPartnerBody));
        }
        (teleport.ask_go_to_partner)()
    }

    fn bring_partner(&self) -> Result<String, String> {
        match self.teleport {
            Some(teleport) => (teleport.ask_bring_partner)(),
            None => Err(NO_PARTNER_YET_TEXT.to_owned()),
        }
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
            (self.story_status)(),
            self.travel_view().unwrap_or_default().status_line(),
            (self.partner_body_status)(),
        ]
    }

    fn extra_panels(&self) -> Vec<ExtraPanel> {
        Vec::new()
    }

    fn section_notes(&self, section: SectionKind) -> Vec<MenuNote> {
        match (section, self.travel_view()) {
            (SectionKind::Teleport, Some(view)) => {
                view.notes().into_iter().map(MenuNote::plain).collect()
            }
            _ => Vec::new(),
        }
    }
}

fn area_of(point: TravelPoint) -> AreaEntry {
    AreaEntry {
        id: u32::from(point.index()),
        name: point.label(),
    }
}

fn refusal_of(refusal: JumpRefusal) -> AreaChangeRefusal {
    match refusal {
        JumpRefusal::NotHost => AreaChangeRefusal::NotHost,
        other => AreaChangeRefusal::Other(other.to_string()),
    }
}
