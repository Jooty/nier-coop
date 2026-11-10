use crate::menu::{
    AreaEntry, CoOpStatus, ExtraPanel, FixEntry, GameMenu, MenuNote, StoryJumpEntry,
};

pub const SECTION_ORDER: [SectionKind; 5] = [
    SectionKind::CoOp,
    SectionKind::Teleport,
    SectionKind::StoryJump,
    SectionKind::Fixes,
    SectionKind::SettingsAndLogs,
];

const NO_PLAYERS_NOTE: &str = "no other player is connected";
const NO_AREAS_NOTE: &str = "this game lists no areas";
const NO_STORY_JUMPS_NOTE: &str = "no story jumps are available";
const NO_FIXES_NOTE: &str = "this game has no fix actions";
const NO_STATUS_NOTE: &str = "no status lines or extra panels";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionKind {
    CoOp,
    Teleport,
    StoryJump,
    Fixes,
    SettingsAndLogs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionBody {
    CoOp(CoOpStatus),
    Teleport {
        current: Option<AreaEntry>,
        areas: Vec<AreaEntry>,
    },
    StoryJump(Vec<StoryJumpEntry>),
    Fixes(Vec<FixEntry>),
    SettingsAndLogs {
        status_lines: Vec<String>,
        panels: Vec<ExtraPanel>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuSection {
    pub kind: SectionKind,
    pub body: SectionBody,
    pub empty_note: Option<String>,
    pub notes: Vec<MenuNote>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuModel {
    pub sections: Vec<MenuSection>,
}

impl SectionKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::CoOp => "Co-op",
            Self::Teleport => "Teleport",
            Self::StoryJump => "Story jump",
            Self::Fixes => "Fixes",
            Self::SettingsAndLogs => "Settings and logs",
        }
    }
}

pub fn build_menu(game: &dyn GameMenu) -> MenuModel {
    let sections = SECTION_ORDER
        .iter()
        .map(|kind| build_section(game, *kind))
        .collect();
    MenuModel { sections }
}

fn build_section(game: &dyn GameMenu, kind: SectionKind) -> MenuSection {
    let (body, empty_note) = match kind {
        SectionKind::CoOp => {
            let status = game.co_op_status();
            let empty = status
                .players
                .is_empty()
                .then(|| NO_PLAYERS_NOTE.to_owned());
            (SectionBody::CoOp(status), empty)
        }
        SectionKind::Teleport => {
            let areas = game.areas();
            let empty = areas.is_empty().then(|| NO_AREAS_NOTE.to_owned());
            (
                SectionBody::Teleport {
                    current: game.current_area(),
                    areas,
                },
                empty,
            )
        }
        SectionKind::StoryJump => {
            let jumps = game.story_jumps();
            let empty = jumps.is_empty().then(|| NO_STORY_JUMPS_NOTE.to_owned());
            (SectionBody::StoryJump(jumps), empty)
        }
        SectionKind::Fixes => {
            let fixes = game.fixes();
            let empty = fixes.is_empty().then(|| NO_FIXES_NOTE.to_owned());
            (SectionBody::Fixes(fixes), empty)
        }
        SectionKind::SettingsAndLogs => {
            let status_lines = game.status_lines();
            let panels = game.extra_panels();
            let empty =
                (status_lines.is_empty() && panels.is_empty()).then(|| NO_STATUS_NOTE.to_owned());
            (
                SectionBody::SettingsAndLogs {
                    status_lines,
                    panels,
                },
                empty,
            )
        }
    };
    MenuSection {
        kind,
        body,
        empty_note,
        notes: game.section_notes(kind),
    }
}
