use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use crate::logs::rotate_log;

pub struct LogWriter {
    file: File,
}

impl LogWriter {
    pub fn start(dir: &Path, name: &str) -> io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(name);
        rotate_log(&path)?;
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self { file })
    }

    pub fn write_line(&mut self, line: &str) -> io::Result<()> {
        writeln!(self.file, "{line}")?;
        self.file.flush()
    }
}
