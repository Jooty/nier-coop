use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Ledger {
    path: PathBuf,
}

impl Ledger {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }

    pub fn contains(&self, hash: &str) -> bool {
        std::fs::read_to_string(&self.path)
            .map(|text| text.lines().any(|line| line.trim() == hash))
            .unwrap_or(false)
    }

    pub fn record(&self, hash: &str) -> std::io::Result<()> {
        if self.contains(hash) {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{hash}")
    }
}
