use crate::character::Character;
use crate::start::{ChapterRecord, buddy_flag, hero_flag, route_flag};

const FIRST_CHAPTER_GROUP: u8 = 1;
const CLEAR_BIT_COUNT: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartCall {
    WriteHeroModelId(u32),
    SetFlag(String),
    RunMarkerCall,
    ClearFlagBit(u32),
    WriteStartedFromChapterSelect,
    ClearTopBitOfUiBookkeepingWord,
    WriteUiBookkeepingFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseStartCall {
    pub phase_id: i32,
    pub phase_name: String,
    pub chapter_filter: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartPlan {
    pub before_phase_start: Vec<StartCall>,
    pub phase_start: PhaseStartCall,
    pub after_phase_start: Vec<StartCall>,
}

impl StartCall {
    pub fn describe(&self) -> String {
        match self {
            Self::WriteHeroModelId(id) => format!("write Hero::modelId {id:#X}"),
            Self::SetFlag(name) => format!("set scene flag {name}"),
            Self::RunMarkerCall => {
                "run the chapter marker call (group 1 and the first hero only)".to_owned()
            }
            Self::ClearFlagBit(bit) => format!("clear clear-flag bit {bit}"),
            Self::WriteStartedFromChapterSelect => {
                "write the started-from-chapter-select mark 1".to_owned()
            }
            Self::ClearTopBitOfUiBookkeepingWord => "clear the top bit of the UI word".to_owned(),
            Self::WriteUiBookkeepingFlag => "write the UI bookkeeping flag 1".to_owned(),
        }
    }
}

pub fn build_plan(record: &ChapterRecord, hero: Character, partner: Character) -> StartPlan {
    let mut before = vec![
        StartCall::WriteHeroModelId(hero.object_id()),
        StartCall::SetFlag(hero_flag(hero).to_owned()),
        StartCall::SetFlag(buddy_flag(partner).to_owned()),
        StartCall::SetFlag(route_flag(record.route, hero).to_owned()),
    ];
    if record.group == FIRST_CHAPTER_GROUP && hero == Character::TwoB {
        before.push(StartCall::RunMarkerCall);
    }
    before.extend((0..CLEAR_BIT_COUNT).map(StartCall::ClearFlagBit));
    before.push(StartCall::WriteStartedFromChapterSelect);
    StartPlan {
        before_phase_start: before,
        phase_start: PhaseStartCall {
            phase_id: i32::from(record.phase_id),
            phase_name: record.phase_name.clone(),
            chapter_filter: i32::from(record.chapter) + 1,
        },
        after_phase_start: vec![
            StartCall::ClearTopBitOfUiBookkeepingWord,
            StartCall::WriteUiBookkeepingFlag,
        ],
    }
}
