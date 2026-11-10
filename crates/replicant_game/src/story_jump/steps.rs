use replicant_proto::StoryJumpPoint;

const AFTER_PROLOGUE_STEPS: [JumpStep; 18] = [
    JumpStep::SetFlag { id: 71 },
    JumpStep::SetFlag { id: 153 },
    JumpStep::ClearFlag { id: 1403 },
    JumpStep::ClearFlag { id: 149 },
    JumpStep::SetTutorial { id: 2000 },
    JumpStep::SetTutorial { id: 2012 },
    JumpStep::SetTutorial { id: 2026 },
    JumpStep::SetTutorial { id: 2074 },
    JumpStep::SetTutorial { id: 2386 },
    JumpStep::SetTutorial { id: 2387 },
    JumpStep::SetTutorial { id: 2388 },
    JumpStep::SetTutorial { id: 2389 },
    JumpStep::SetTutorial { id: 2719 },
    JumpStep::SetTutorial { id: 2732 },
    JumpStep::SetItem { id: 727, count: 0 },
    JumpStep::SetItem { id: 1000, count: 1 },
    JumpStep::EquipWeapon { id: 1000 },
    JumpStep::SetPlayerLevel { level: 0 },
];

const POPOLA_TALK_EXTRA_STEPS: [JumpStep; 3] = [
    JumpStep::SetFlag { id: 154 },
    JumpStep::SetFlag { id: 1299 },
    JumpStep::SetFlag { id: 1401 },
];

const BEFORE_NEXT_DAY_EXTRA_STEPS: [JumpStep; 21] = [
    JumpStep::SetFlag { id: 154 },
    JumpStep::SetFlag { id: 73 },
    JumpStep::SetFlag { id: 156 },
    JumpStep::SetFlag { id: 160 },
    JumpStep::SetFlag { id: 161 },
    JumpStep::SetFlag { id: 157 },
    JumpStep::SetFlag { id: 1360 },
    JumpStep::SetFlag { id: 158 },
    JumpStep::SetFlag { id: 112 },
    JumpStep::SetFlag { id: 159 },
    JumpStep::SetFlag { id: 162 },
    JumpStep::SetFlag { id: 1443 },
    JumpStep::SetFlag { id: 165 },
    JumpStep::SetFlag { id: 163 },
    JumpStep::SetFlag { id: 164 },
    JumpStep::SetFlag { id: 166 },
    JumpStep::SetFlag { id: 1300 },
    JumpStep::ClearFlag { id: 1299 },
    JumpStep::ClearFlag { id: 1401 },
    JumpStep::SetItem { id: 723, count: 1 },
    JumpStep::SetTutorial { id: 2010 },
];

const AT_SHRINE_ENTRANCE_EXTRA_STEPS: [JumpStep; 32] = [
    JumpStep::SetFlag { id: 167 },
    JumpStep::SetFlag { id: 168 },
    JumpStep::SetFlag { id: 169 },
    JumpStep::SetSnowFlag { id: 1052 },
    JumpStep::SetSnowFlag { id: 1069 },
    JumpStep::SetFlag { id: 170 },
    JumpStep::SetFlag { id: 171 },
    JumpStep::SetFlag { id: 172 },
    JumpStep::SetFlag { id: 173 },
    JumpStep::SetFlag { id: 174 },
    JumpStep::SetFlag { id: 175 },
    JumpStep::SetFlag { id: 176 },
    JumpStep::SetFlag { id: 189 },
    JumpStep::SetFlag { id: 190 },
    JumpStep::SetFlag { id: 191 },
    JumpStep::SetFlag { id: 116 },
    JumpStep::SetFlag { id: 77 },
    JumpStep::SetFlag { id: 1303 },
    JumpStep::ClearFlag { id: 1300 },
    JumpStep::ClearFlag { id: 1443 },
    JumpStep::ClearFlag { id: 1401 },
    JumpStep::ClearFlag { id: 1402 },
    JumpStep::ClearFlag { id: 1403 },
    JumpStep::ClearFlag { id: 1404 },
    JumpStep::ClearFlag { id: 1405 },
    JumpStep::ClearFlag { id: 1406 },
    JumpStep::ClearFlag { id: 1407 },
    JumpStep::ClearFlag { id: 1408 },
    JumpStep::ClearFlag { id: 1409 },
    JumpStep::ClearFlag { id: 1410 },
    JumpStep::SetGameCounter {
        id: 100,
        value: 255,
    },
    JumpStep::SetGameCounter {
        id: 101,
        value: 255,
    },
];

const AT_SHRINE_TOP_EXTRA_STEPS: [JumpStep; 7] = [
    JumpStep::SetFlag { id: 75 },
    JumpStep::SetFlag { id: 192 },
    JumpStep::SetFlag { id: 138 },
    JumpStep::SetFlag { id: 193 },
    JumpStep::SetFlag { id: 1432 },
    JumpStep::SetFlag { id: 205 },
    JumpStep::SetItem { id: 305, count: 0 },
];

const AT_SHRINE_BOSS_EXTRA_STEPS: [JumpStep; 3] = [
    JumpStep::SetFlag { id: 194 },
    JumpStep::SetSnowFlag { id: 1053 },
    JumpStep::SetFlag { id: 206 },
];

const AT_SHRINE_ARENA_EXTRA_STEPS: [JumpStep; 9] = [
    JumpStep::ClearFlag { id: 1303 },
    JumpStep::SetFlag { id: 1432 },
    JumpStep::SetFlag { id: 1433 },
    JumpStep::SetFlag { id: 1434 },
    JumpStep::SetFlag { id: 127 },
    JumpStep::SetHaveWhiteBook { on: true },
    JumpStep::LearnMagic { id: 3 },
    JumpStep::SetMagicSlot { slot: 1, magic: 3 },
    JumpStep::SetTutorial { id: 2002 },
];

const HOME_AFTER_SHRINE_BOSS_EXTRA_STEPS: [JumpStep; 22] = [
    JumpStep::SetFlag { id: 195 },
    JumpStep::SetFlag { id: 196 },
    JumpStep::SetFlag { id: 198 },
    JumpStep::SetFlag { id: 204 },
    JumpStep::SetFlag { id: 130 },
    JumpStep::LearnMagic { id: 6 },
    JumpStep::SetMagicSlot { slot: 0, magic: 6 },
    JumpStep::SetTutorial { id: 2005 },
    JumpStep::SetTutorial { id: 2020 },
    JumpStep::SetTutorial { id: 2079 },
    JumpStep::SetFlag { id: 200 },
    JumpStep::SetFlag { id: 201 },
    JumpStep::ClearFlag { id: 1403 },
    JumpStep::SetFlag { id: 202 },
    JumpStep::SetGameCounter { id: 100, value: 3 },
    JumpStep::SetGameCounter { id: 101, value: 3 },
    JumpStep::SetFlag { id: 217 },
    JumpStep::SetFlag { id: 219 },
    JumpStep::SetFlag { id: 1304 },
    JumpStep::SetFlag { id: 1361 },
    JumpStep::SetFlag { id: 218 },
    JumpStep::SetSnowFlag { id: 1085 },
];

const AFTER_PROLOGUE_MP_STEP: JumpStep = JumpStep::MpFullMode { on: false };
const AFTER_PROLOGUE_PHASE: &str = "A_CENTER_VILLAGE_01";
const AFTER_PROLOGUE_POINT: i32 = 8;
const POPOLA_TALK_PHASE: &str = "A_CENTER_LIBRARY_01";
const POPOLA_TALK_POINT: i32 = 1;
const BEFORE_NEXT_DAY_PHASE: &str = "A_CENTER_VILLAGE_01";
const BEFORE_NEXT_DAY_POINT: i32 = 4;
const AT_SHRINE_ENTRANCE_PHASE: &str = "A_STONE_TEMPLE_01";
const AT_SHRINE_ENTRANCE_POINT: i32 = 1;
const AT_SHRINE_TOP_PHASE: &str = "A_STONE_FIELD_01";
const AT_SHRINE_TOP_POINT: i32 = 3;
const AT_SHRINE_BOSS_PHASE: &str = "A_STONE_FIELD_02";
const AT_SHRINE_BOSS_POINT: i32 = 1;
const HOME_AFTER_SHRINE_BOSS_PHASE: &str = "A_CENTER_VILLAGE_01";
const HOME_AFTER_SHRINE_BOSS_POINT: i32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpStep {
    SetFlag { id: u16 },
    ClearFlag { id: u16 },
    SetTutorial { id: u16 },
    SetSnowFlag { id: u16 },
    SetGameCounter { id: u8, value: u8 },
    SetItem { id: i32, count: i32 },
    EquipWeapon { id: i32 },
    SetPlayerLevel { level: i32 },
    MpFullMode { on: bool },
    SetHaveWhiteBook { on: bool },
    LearnMagic { id: i32 },
    SetMagicSlot { slot: i32, magic: i32 },
}

pub fn loadout_steps(point: StoryJumpPoint) -> Vec<JumpStep> {
    match point {
        StoryJumpPoint::AfterPrologue => {
            let mut steps = AFTER_PROLOGUE_STEPS.to_vec();
            steps.push(AFTER_PROLOGUE_MP_STEP);
            steps
        }
        StoryJumpPoint::AtPopolaTalk => {
            let mut steps = AFTER_PROLOGUE_STEPS.to_vec();
            steps.extend_from_slice(&POPOLA_TALK_EXTRA_STEPS);
            steps.push(AFTER_PROLOGUE_MP_STEP);
            steps
        }
        StoryJumpPoint::BeforeNextDay => {
            let mut steps = AFTER_PROLOGUE_STEPS.to_vec();
            steps.extend_from_slice(&BEFORE_NEXT_DAY_EXTRA_STEPS);
            steps.push(AFTER_PROLOGUE_MP_STEP);
            steps
        }
        StoryJumpPoint::AtShrineEntrance => {
            let mut steps = AFTER_PROLOGUE_STEPS.to_vec();
            steps.extend_from_slice(&BEFORE_NEXT_DAY_EXTRA_STEPS);
            steps.extend_from_slice(&AT_SHRINE_ENTRANCE_EXTRA_STEPS);
            steps.push(AFTER_PROLOGUE_MP_STEP);
            steps
        }
        StoryJumpPoint::AtShrineTop => {
            let mut steps = AFTER_PROLOGUE_STEPS.to_vec();
            steps.extend_from_slice(&BEFORE_NEXT_DAY_EXTRA_STEPS);
            steps.extend_from_slice(&AT_SHRINE_ENTRANCE_EXTRA_STEPS);
            steps.extend_from_slice(&AT_SHRINE_TOP_EXTRA_STEPS);
            steps.push(AFTER_PROLOGUE_MP_STEP);
            steps
        }
        StoryJumpPoint::AtShrineBoss => {
            let mut steps = AFTER_PROLOGUE_STEPS.to_vec();
            steps.extend_from_slice(&BEFORE_NEXT_DAY_EXTRA_STEPS);
            steps.extend_from_slice(&AT_SHRINE_ENTRANCE_EXTRA_STEPS);
            steps.extend_from_slice(&AT_SHRINE_TOP_EXTRA_STEPS);
            steps.extend_from_slice(&AT_SHRINE_BOSS_EXTRA_STEPS);
            steps.extend_from_slice(&AT_SHRINE_ARENA_EXTRA_STEPS);
            steps.push(AFTER_PROLOGUE_MP_STEP);
            steps
        }
        StoryJumpPoint::HomeAfterShrineBoss => {
            let mut steps = loadout_steps(StoryJumpPoint::AtShrineBoss);
            steps.pop();
            steps.extend_from_slice(&HOME_AFTER_SHRINE_BOSS_EXTRA_STEPS);
            steps.push(AFTER_PROLOGUE_MP_STEP);
            steps
        }
    }
}

pub fn destination(point: StoryJumpPoint) -> (&'static str, i32) {
    match point {
        StoryJumpPoint::AfterPrologue => (AFTER_PROLOGUE_PHASE, AFTER_PROLOGUE_POINT),
        StoryJumpPoint::AtPopolaTalk => (POPOLA_TALK_PHASE, POPOLA_TALK_POINT),
        StoryJumpPoint::BeforeNextDay => (BEFORE_NEXT_DAY_PHASE, BEFORE_NEXT_DAY_POINT),
        StoryJumpPoint::AtShrineEntrance => (AT_SHRINE_ENTRANCE_PHASE, AT_SHRINE_ENTRANCE_POINT),
        StoryJumpPoint::AtShrineTop => (AT_SHRINE_TOP_PHASE, AT_SHRINE_TOP_POINT),
        StoryJumpPoint::AtShrineBoss => (AT_SHRINE_BOSS_PHASE, AT_SHRINE_BOSS_POINT),
        StoryJumpPoint::HomeAfterShrineBoss => {
            (HOME_AFTER_SHRINE_BOSS_PHASE, HOME_AFTER_SHRINE_BOSS_POINT)
        }
    }
}
