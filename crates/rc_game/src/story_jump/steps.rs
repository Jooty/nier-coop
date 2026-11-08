use rc_proto::StoryJumpPoint;

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

const BEFORE_NEXT_DAY_EXTRA_STEPS: [JumpStep; 20] = [
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
];

const AFTER_PROLOGUE_MP_STEP: JumpStep = JumpStep::MpFullMode { on: false };
const AFTER_PROLOGUE_PHASE: &str = "A_CENTER_VILLAGE_01";
const AFTER_PROLOGUE_POINT: i32 = 8;
const POPOLA_TALK_PHASE: &str = "A_CENTER_LIBRARY_01";
const POPOLA_TALK_POINT: i32 = 1;
const BEFORE_NEXT_DAY_PHASE: &str = "A_CENTER_VILLAGE_01";
const BEFORE_NEXT_DAY_POINT: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpStep {
    SetFlag { id: u16 },
    ClearFlag { id: u16 },
    SetTutorial { id: u16 },
    SetItem { id: i32, count: i32 },
    EquipWeapon { id: i32 },
    SetPlayerLevel { level: i32 },
    MpFullMode { on: bool },
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
    }
}

pub fn destination(point: StoryJumpPoint) -> (&'static str, i32) {
    match point {
        StoryJumpPoint::AfterPrologue => (AFTER_PROLOGUE_PHASE, AFTER_PROLOGUE_POINT),
        StoryJumpPoint::AtPopolaTalk => (POPOLA_TALK_PHASE, POPOLA_TALK_POINT),
        StoryJumpPoint::BeforeNextDay => (BEFORE_NEXT_DAY_PHASE, BEFORE_NEXT_DAY_POINT),
    }
}
