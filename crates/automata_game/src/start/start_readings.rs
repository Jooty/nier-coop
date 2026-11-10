use crate::start::{NO_PENDING_PHASE_ID, TITLE_PHASE_ID, TITLE_SCENE_STATE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TitleReadings {
    pub scene_state: i32,
    pub phase_id: i32,
    pub pending_phase_id: i32,
    pub loading: i32,
    pub save_busy: Option<bool>,
    pub save_failed: Option<bool>,
    pub chapter_data_loaded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrivalReadings {
    pub scene_state: i32,
    pub phase_id: i32,
    pub phase_name: String,
    pub loading: i32,
    pub player_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartReadings {
    pub title: TitleReadings,
    pub arrival: Option<ArrivalReadings>,
    pub slot_file_present: bool,
}

impl TitleReadings {
    pub fn describe(&self) -> String {
        format!(
            "scene state {}, phase {:#X}, pending {}, loading {}, save busy {}, chapter data loaded {}",
            self.scene_state,
            self.phase_id,
            self.pending_phase_id,
            self.loading,
            self.save_busy
                .map_or("not read yet".to_owned(), |busy| busy.to_string()),
            self.chapter_data_loaded
        )
    }
}

impl ArrivalReadings {
    pub fn describe(&self) -> String {
        format!(
            "scene state {}, phase {:#X} {:?}, loading {}, player present {}",
            self.scene_state, self.phase_id, self.phase_name, self.loading, self.player_present
        )
    }
}

pub fn title_is_idle(readings: &TitleReadings) -> bool {
    readings.scene_state == TITLE_SCENE_STATE
        && readings.phase_id == TITLE_PHASE_ID
        && readings.pending_phase_id == NO_PENDING_PHASE_ID
        && readings.loading == 0
        && readings.save_busy == Some(false)
        && readings.chapter_data_loaded
}

pub fn has_arrived(
    readings: &ArrivalReadings,
    chosen_phase_id: i32,
    chosen_phase_name: &str,
) -> bool {
    readings.scene_state == TITLE_SCENE_STATE
        && readings.phase_id == chosen_phase_id
        && readings.phase_name == chosen_phase_name
        && readings.loading == 0
        && readings.player_present
}
