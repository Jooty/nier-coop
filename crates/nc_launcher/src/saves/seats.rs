use std::io;
use std::path::{Path, PathBuf};

use replicant_game::save_files::COOP_GAMEDATA_FILE_NAME;
use replicant_game::story_progress::{SeatError, check_seat};
use thiserror::Error;

use crate::saves::coop_gamedata_path;

const COOP_SEATS_SUFFIX: &str = ".seats";
const SEAT_FILE_EXTENSION: &str = "bin";

#[derive(Debug, Error)]
pub enum SeatFileError {
    #[error("the seat is damaged: {0:?}")]
    Damaged(SeatError),
    #[error("{0}")]
    Io(#[from] io::Error),
}

pub fn coop_seats_dir(save_dir: &Path) -> Option<PathBuf> {
    let coop = coop_gamedata_path(save_dir)?;
    Some(seats_dir_beside(&coop))
}

pub(super) fn seats_dir_beside(coop_file: &Path) -> PathBuf {
    coop_file.with_file_name(format!("{COOP_GAMEDATA_FILE_NAME}{COOP_SEATS_SUFFIX}"))
}

pub fn seat_file_path(seats_dir: &Path, host_slot: u8) -> PathBuf {
    seats_dir.join(format!("{host_slot}.{SEAT_FILE_EXTENSION}"))
}

pub fn store_seat(seats_dir: &Path, host_slot: u8, seat: &[u8]) -> Result<(), SeatFileError> {
    check_seat(seat).map_err(SeatFileError::Damaged)?;
    std::fs::create_dir_all(seats_dir)?;
    let path = seat_file_path(seats_dir, host_slot);
    let staging = path.with_extension("tmp");
    std::fs::write(&staging, seat)?;
    std::fs::rename(&staging, &path)?;
    Ok(())
}

pub fn read_seat(seats_dir: &Path, host_slot: u8) -> Result<Option<Vec<u8>>, SeatFileError> {
    let bytes = match std::fs::read(seat_file_path(seats_dir, host_slot)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    check_seat(&bytes).map_err(SeatFileError::Damaged)?;
    Ok(Some(bytes))
}

pub fn remove_seat(seats_dir: &Path, host_slot: u8) -> io::Result<()> {
    match std::fs::remove_file(seat_file_path(seats_dir, host_slot)) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}
