use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nt_launcher_core::{
    BundleFile, BundleHeader, LOG_FILE_TAIL_CAP_BYTES, LogFileEntry, MAX_UPLOAD_BODY_BYTES,
    UPLOAD_TIMEOUT_SECONDS, UPLOAD_TOKEN_HEADER, build_bundle, gzip_text, parse_sent_marker,
    render_sent_marker,
};

use crate::logs::SENT_MARKER_FILE_NAME;

pub struct UploadJob {
    pub dir: PathBuf,
    pub names: Vec<String>,
    pub header: BundleHeader,
    pub url: String,
    pub token: String,
    pub sent_marker_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadOutcome {
    Sent { kilobytes: usize },
    Failed(String),
}

pub fn unix_seconds(moment: SystemTime) -> u64 {
    moment
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

pub fn list_log_entries(dir: &Path) -> Vec<LogFileEntry> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.filter_map(Result::ok)
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            metadata.is_file().then_some(())?;
            Some(LogFileEntry {
                name: entry.file_name().into_string().ok()?,
                modified_secs: unix_seconds(metadata.modified().ok()?),
            })
        })
        .collect()
}

pub fn read_sent_marker(dir: &Path) -> Option<u64> {
    parse_sent_marker(&std::fs::read_to_string(dir.join(SENT_MARKER_FILE_NAME)).ok()?)
}

pub fn write_sent_marker(dir: &Path, sent_at_secs: u64) -> io::Result<()> {
    std::fs::write(
        dir.join(SENT_MARKER_FILE_NAME),
        render_sent_marker(sent_at_secs),
    )
}

pub fn gzip_bundle_of(
    dir: &Path,
    names: &[String],
    header: &BundleHeader,
    tail_cap_bytes: usize,
) -> io::Result<Vec<u8>> {
    let files: Vec<BundleFile> = names
        .iter()
        .filter_map(|name| {
            Some(BundleFile {
                name: name.clone(),
                bytes: std::fs::read(dir.join(name)).ok()?,
            })
        })
        .collect();
    gzip_text(&build_bundle(header, &files, tail_cap_bytes))
}

pub fn put_logs(url: &str, token: &str, body: &[u8]) -> Result<(), String> {
    if body.len() > MAX_UPLOAD_BODY_BYTES {
        return Err("the logs are too large to send".to_owned());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(UPLOAD_TIMEOUT_SECONDS)))
        .build()
        .into();
    agent
        .put(url)
        .header(UPLOAD_TOKEN_HEADER, token)
        .header("Content-Type", "application/gzip")
        .send(body)
        .map(|_| ())
        .map_err(|error| match error {
            ureq::Error::StatusCode(code) => format!("the server answered {code}"),
            other => other.to_string(),
        })
}

pub fn run_upload(job: &UploadJob) -> UploadOutcome {
    let body = match gzip_bundle_of(&job.dir, &job.names, &job.header, LOG_FILE_TAIL_CAP_BYTES) {
        Ok(body) => body,
        Err(error) => return UploadOutcome::Failed(error.to_string()),
    };
    if let Err(reason) = put_logs(&job.url, &job.token, &body) {
        return UploadOutcome::Failed(reason);
    }
    let _ = write_sent_marker(&job.dir, job.sent_marker_secs);
    UploadOutcome::Sent {
        kilobytes: body.len().div_ceil(1024),
    }
}
