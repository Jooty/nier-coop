use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use rc_game::log_stamp::{header_line, role_of_log_name, stamped_line, unix_millis_now};

use crate::logs::rotate_log;
use crate::update::BUILD_COMMIT;

pub struct LogWriter {
    file: File,
}

impl LogWriter {
    pub fn start(dir: &Path, name: &str) -> io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(name);
        rotate_log(&path)?;
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let mut writer = Self { file };
        writeln!(
            writer.file,
            "{}",
            header_line(unix_millis_now(), BUILD_COMMIT, role_of_log_name(name))
        )?;
        Ok(writer)
    }

    pub fn write_line(&mut self, line: &str) -> io::Result<()> {
        writeln!(self.file, "{}", stamped_line(unix_millis_now(), line))?;
        self.file.flush()
    }
}
