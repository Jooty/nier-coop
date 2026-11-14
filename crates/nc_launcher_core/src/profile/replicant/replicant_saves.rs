use replicant_game::save_files::{COOP_GAMEDATA_FILE_NAME, CoopSaveName, GAMEDATA_FILE_NAME};
use replicant_game::story_progress::{self, SLOT_BYTES};

use crate::launcher_game::{GameSaves, SeatError};

#[derive(Debug)]
pub struct ReplicantSaves;

pub const REPLICANT_SAVES: ReplicantSaves = ReplicantSaves;

impl GameSaves for ReplicantSaves {
    fn vanilla_file_name(&self) -> &'static str {
        GAMEDATA_FILE_NAME
    }

    fn coop_file_name(&self) -> &'static str {
        COOP_GAMEDATA_FILE_NAME
    }

    fn is_coop_session_file_name(&self, name: &str) -> bool {
        CoopSaveName::parse(Some(name)).is_ok()
    }

    fn seat_slot_bytes(&self) -> usize {
        SLOT_BYTES
    }

    fn check_seat(&self, seat: &[u8]) -> Result<(), SeatError> {
        story_progress::check_seat(seat).map_err(seat_error_of)
    }

    fn changed_slot(&self, before: &[u8], after: &[u8]) -> Option<u8> {
        story_progress::changed_slot(before, after)
    }

    fn seat_bytes_of(&self, file: &[u8], slot: u8) -> Result<Vec<u8>, SeatError> {
        story_progress::seat_bytes_of(file, slot).map_err(seat_error_of)
    }

    fn session_file_with_seat(&self, blank: &[u8], seat: &[u8]) -> Result<Vec<u8>, SeatError> {
        story_progress::session_file_with_seat(blank, seat).map_err(seat_error_of)
    }

    fn fix_header_checksum(&self, file: &mut [u8]) -> bool {
        story_progress::fix_header_checksum(file).is_ok()
    }
}

fn seat_error_of(error: story_progress::SeatError) -> SeatError {
    match error {
        story_progress::SeatError::FileSizeWrong => SeatError::FileSizeWrong,
        story_progress::SeatError::SlotSizeWrong => SeatError::SlotSizeWrong,
        story_progress::SeatError::NoSuchSlot => SeatError::NoSuchSlot,
        story_progress::SeatError::NotAGameSlot => SeatError::NotAGameSlot,
        story_progress::SeatError::EmptySlot => SeatError::EmptySlot,
        story_progress::SeatError::ChecksumWrong => SeatError::ChecksumWrong,
    }
}
