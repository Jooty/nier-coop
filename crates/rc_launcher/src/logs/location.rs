use std::path::PathBuf;

use crate::config::data_dir;
use crate::second_copy::LOGS_FOLDER_NAME;

pub fn logs_dir() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join(LOGS_FOLDER_NAME))
}
