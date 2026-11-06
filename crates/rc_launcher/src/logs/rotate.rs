use std::io;
use std::path::{Path, PathBuf};

use crate::logs::LOG_FILES_KEPT;

pub fn rotate_log(path: &Path) -> io::Result<()> {
    let numbered = |number: usize| {
        let mut name = path.as_os_str().to_owned();
        name.push(format!(".{number}"));
        PathBuf::from(name)
    };
    let oldest = LOG_FILES_KEPT - 1;
    let _ = std::fs::remove_file(numbered(oldest));
    for number in (1..oldest).rev() {
        if numbered(number).exists() {
            std::fs::rename(numbered(number), numbered(number + 1))?;
        }
    }
    if path.exists() {
        std::fs::rename(path, numbered(1))?;
    }
    Ok(())
}
