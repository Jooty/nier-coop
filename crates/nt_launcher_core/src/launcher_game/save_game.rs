use crate::launcher_game::{GameSaves, SeatError};
use crate::profile::ALL_GAMES;

const NO_SAVE_GAME_FILE_NAME: &str = "";
const NO_SAVE_GAME_SEAT_BYTES: usize = 0;

pub fn save_game() -> Option<&'static dyn GameSaves> {
    ALL_GAMES.iter().find_map(|game| game.saves())
}

pub fn vanilla_file_name() -> &'static str {
    save_game().map_or(NO_SAVE_GAME_FILE_NAME, |saves| saves.vanilla_file_name())
}

pub fn coop_file_name() -> &'static str {
    save_game().map_or(NO_SAVE_GAME_FILE_NAME, |saves| saves.coop_file_name())
}

pub fn is_coop_session_file_name(name: &str) -> bool {
    save_game().is_some_and(|saves| saves.is_coop_session_file_name(name))
}

pub fn seat_slot_bytes() -> usize {
    save_game().map_or(NO_SAVE_GAME_SEAT_BYTES, |saves| saves.seat_slot_bytes())
}

pub fn check_seat(seat: &[u8]) -> Result<(), SeatError> {
    save_game().map_or(Err(SeatError::GameKeepsNoSaves), |saves| {
        saves.check_seat(seat)
    })
}

pub fn changed_slot(before: &[u8], after: &[u8]) -> Option<u8> {
    save_game()?.changed_slot(before, after)
}

pub fn seat_bytes_of(file: &[u8], slot: u8) -> Result<Vec<u8>, SeatError> {
    save_game().map_or(Err(SeatError::GameKeepsNoSaves), |saves| {
        saves.seat_bytes_of(file, slot)
    })
}

pub fn session_file_with_seat(blank: &[u8], seat: &[u8]) -> Result<Vec<u8>, SeatError> {
    save_game().map_or(Err(SeatError::GameKeepsNoSaves), |saves| {
        saves.session_file_with_seat(blank, seat)
    })
}

pub fn fix_header_checksum(file: &mut [u8]) -> bool {
    save_game().is_some_and(|saves| saves.fix_header_checksum(file))
}
