use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread::JoinHandle;

use nc_launcher::config::LauncherConfig;
use nc_launcher::logs::{
    UploadJob, UploadOutcome, gzip_bundle_of, list_log_entries, put_logs, read_sent_marker,
    run_upload, write_sent_marker,
};
use nc_launcher_core::{BundleHeader, gunzip_text};

use crate::scratch::fresh_dir;

struct SeenRequest {
    request_line: String,
    headers: String,
    body: Vec<u8>,
}

fn serve_once(status_line: &'static str) -> (String, JoinHandle<()>, mpsc::Receiver<SeenRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sender, receiver) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut raw = Vec::new();
        let mut chunk = [0u8; 4096];
        let split = loop {
            let read = stream.read(&mut chunk).unwrap();
            raw.extend_from_slice(&chunk[..read]);
            if let Some(at) = raw.windows(4).position(|window| window == b"\r\n\r\n") {
                break at + 4;
            }
        };
        let head = String::from_utf8_lossy(&raw[..split]).to_string();
        let length: usize = head
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(|value| value.trim().parse().unwrap())
            })
            .unwrap_or(0);
        while raw.len() < split + length {
            let read = stream.read(&mut chunk).unwrap();
            raw.extend_from_slice(&chunk[..read]);
        }
        let mut lines = head.lines();
        let request_line = lines.next().unwrap().to_owned();
        let headers = lines.collect::<Vec<_>>().join("\n").to_ascii_lowercase();
        sender
            .send(SeenRequest {
                request_line,
                headers,
                body: raw[split..split + length].to_vec(),
            })
            .unwrap();
        let reply =
            format!("HTTP/1.1 {status_line}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        stream.write_all(reply.as_bytes()).unwrap();
    });
    (format!("http://{address}"), handle, receiver)
}

fn header() -> BundleHeader {
    BundleHeader {
        install_id: "0123456789abcdef0123456789abcdef".to_owned(),
        player_name: "Ayla".to_owned(),
        role: "host".to_owned(),
        build_commit: "abc1234".to_owned(),
        os: "linux".to_owned(),
        launcher_started: "2026-11-07 10:00:00 UTC".to_owned(),
    }
}

fn job(dir: &std::path::Path, origin: &str) -> UploadJob {
    UploadJob {
        dir: dir.to_path_buf(),
        names: vec!["game-host.log".to_owned()],
        header: header(),
        url: format!("{origin}/logs/0123456789abcdef0123456789abcdef/20261107T100000Z-host.log.gz"),
        token: "secret-test-token".to_owned(),
        sent_marker_secs: 1234,
    }
}

#[test]
fn the_put_carries_the_token_and_a_body_that_gunzips_to_the_bundle() {
    let dir = fresh_dir("upload_put");
    std::fs::write(dir.join("game-host.log"), "line one\nline two\n").unwrap();
    let (origin, handle, seen) = serve_once("201 Created");
    let outcome = run_upload(&job(&dir, &origin));
    handle.join().unwrap();
    assert!(matches!(outcome, UploadOutcome::Sent { kilobytes: 1 }));
    let request = seen.recv().unwrap();
    assert_eq!(
        request.request_line,
        "PUT /logs/0123456789abcdef0123456789abcdef/20261107T100000Z-host.log.gz HTTP/1.1"
    );
    assert!(
        request
            .headers
            .contains("x-upload-token: secret-test-token")
    );
    let expected = gzip_bundle_of(&dir, &["game-host.log".to_owned()], &header(), 1024).unwrap();
    assert_eq!(
        gunzip_text(&request.body).unwrap(),
        gunzip_text(&expected).unwrap()
    );
    assert!(
        gunzip_text(&request.body)
            .unwrap()
            .contains("===== game-host.log (18 bytes) =====\nline one\nline two\n")
    );
}

#[test]
fn a_204_answer_counts_as_sent_and_writes_the_marker() {
    let dir = fresh_dir("upload_204");
    std::fs::write(dir.join("game-host.log"), "x\n").unwrap();
    let (origin, handle, _seen) = serve_once("204 No Content");
    let outcome = run_upload(&job(&dir, &origin));
    handle.join().unwrap();
    assert!(matches!(outcome, UploadOutcome::Sent { .. }));
    assert_eq!(read_sent_marker(&dir), Some(1234));
}

#[test]
fn a_refused_upload_reports_the_status_and_leaves_no_marker() {
    let dir = fresh_dir("upload_403");
    std::fs::write(dir.join("game-host.log"), "x\n").unwrap();
    let (origin, handle, _seen) = serve_once("403 Forbidden");
    let outcome = run_upload(&job(&dir, &origin));
    handle.join().unwrap();
    assert_eq!(
        outcome,
        UploadOutcome::Failed("the server answered 403".to_owned())
    );
    assert_eq!(read_sent_marker(&dir), None);
}

#[test]
fn an_unreachable_server_is_a_failure_not_a_panic() {
    let (origin, handle, _seen) = serve_once("200 OK");
    let unused = TcpListener::bind("127.0.0.1:0").unwrap();
    let dead = format!("http://{}", unused.local_addr().unwrap());
    drop(unused);
    assert!(put_logs(&format!("{dead}/logs/x"), "t", b"body").is_err());
    put_logs(&format!("{origin}/logs/x"), "t", b"body").unwrap();
    handle.join().unwrap();
}

#[test]
fn a_body_over_the_server_limit_is_not_sent() {
    let body = vec![0u8; 16 * 1024 * 1024 + 1];
    assert_eq!(
        put_logs("http://127.0.0.1:1/logs/x", "t", &body),
        Err("the logs are too large to send".to_owned())
    );
}

#[test]
fn the_sent_marker_is_kept_in_the_logs_folder_and_never_listed_as_a_run_log() {
    let dir = fresh_dir("upload_marker");
    std::fs::write(dir.join("game-guest.log"), "x").unwrap();
    assert_eq!(read_sent_marker(&dir), None);
    write_sent_marker(&dir, 99).unwrap();
    assert_eq!(read_sent_marker(&dir), Some(99));
    let names: Vec<String> = list_log_entries(&dir)
        .into_iter()
        .map(|entry| entry.name)
        .filter(|name| nc_launcher_core::is_run_log_name(name))
        .collect();
    assert_eq!(names, ["game-guest.log"]);
}

#[test]
fn sending_logs_is_on_by_default_and_the_install_id_starts_empty() {
    let config = LauncherConfig::default();
    assert!(config.send_logs);
    assert_eq!(config.install_id, None);
    let off = LauncherConfig::parse("send_logs = false\ninstall_id = \"abc\"\n");
    assert!(!off.send_logs);
    assert_eq!(off.install_id.as_deref(), Some("abc"));
}
