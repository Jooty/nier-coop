use crate::start::{
    RUNNING_SCENE_STATE, SCENE_LOADED_FLAG, SUB_PHASE_NAME_BUFFER_LEN, TITLE_PHASE_ID,
    TITLE_START_SUB_PHASE_NAME,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubPhaseReading {
    Name(String),
    NotAName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleIdle {
    No,
    Yes,
    YesButSubPhaseIsNotAName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleReadings {
    pub scene_state: i32,
    pub phase_id: i32,
    pub pending_phase_id: i32,
    pub scene_loaded: i32,
    pub sub_phase: SubPhaseReading,
    pub save_busy: Option<bool>,
    pub save_failed: Option<bool>,
    pub chapter_data_loaded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrivalReadings {
    pub scene_state: i32,
    pub phase_id: i32,
    pub phase_name: String,
    pub scene_loaded: i32,
    pub player_present: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayReadings {
    pub scene_state: i32,
    pub phase_id: i32,
    pub scene_loaded: i32,
    pub player_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartReadings {
    pub title: TitleReadings,
    pub arrival: Option<ArrivalReadings>,
    pub slot_file_present: bool,
    pub slot_calls_verified: bool,
}

impl SubPhaseReading {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        let buffer = &bytes[..bytes.len().min(SUB_PHASE_NAME_BUFFER_LEN)];
        let Some(len) = buffer.iter().position(|byte| *byte == 0) else {
            return Self::NotAName;
        };
        let name = &buffer[..len];
        if name.is_empty() || !name.iter().all(u8::is_ascii_graphic) {
            return Self::NotAName;
        }
        Self::Name(String::from_utf8_lossy(name).into_owned())
    }

    pub fn describe(&self) -> String {
        match self {
            Self::Name(name) => name.clone(),
            Self::NotAName => "not a name".to_owned(),
        }
    }
}

impl TitleReadings {
    pub fn describe(&self) -> String {
        format!(
            "scene state {}, phase {:#X}, sub-phase {}, pending {:#X}, scene loaded {}, save busy {}, chapter data loaded {}",
            self.scene_state,
            self.phase_id,
            self.sub_phase.describe(),
            self.pending_phase_id,
            self.scene_loaded,
            self.save_busy
                .map_or("not read yet".to_owned(), |busy| busy.to_string()),
            self.chapter_data_loaded
        )
    }
}

impl ArrivalReadings {
    pub fn describe(&self) -> String {
        format!(
            "scene state {}, phase {:#X} {:?}, scene loaded {}, player present {}",
            self.scene_state,
            self.phase_id,
            self.phase_name,
            self.scene_loaded,
            self.player_present
        )
    }
}

pub fn title_idle(readings: &TitleReadings) -> TitleIdle {
    let running_title = readings.scene_state == RUNNING_SCENE_STATE
        && readings.phase_id == TITLE_PHASE_ID
        && readings.pending_phase_id == readings.phase_id
        && readings.scene_loaded == SCENE_LOADED_FLAG
        && readings.save_busy == Some(false)
        && readings.chapter_data_loaded;
    if !running_title {
        return TitleIdle::No;
    }
    match &readings.sub_phase {
        SubPhaseReading::Name(name) if name == TITLE_START_SUB_PHASE_NAME => TitleIdle::Yes,
        SubPhaseReading::Name(_) => TitleIdle::No,
        SubPhaseReading::NotAName => TitleIdle::YesButSubPhaseIsNotAName,
    }
}

pub fn title_is_idle(readings: &TitleReadings) -> bool {
    title_idle(readings) == TitleIdle::Yes
}

pub fn has_arrived(readings: &ArrivalReadings, chosen_phase_id: i32) -> bool {
    readings.scene_state == RUNNING_SCENE_STATE
        && readings.phase_id == chosen_phase_id
        && readings.scene_loaded == SCENE_LOADED_FLAG
        && readings.player_present
}

pub fn is_in_play(readings: &PlayReadings) -> bool {
    readings.scene_state == RUNNING_SCENE_STATE
        && readings.phase_id != TITLE_PHASE_ID
        && readings.scene_loaded == SCENE_LOADED_FLAG
        && readings.player_present
}
