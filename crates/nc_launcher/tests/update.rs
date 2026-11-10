use std::io::{Read, Write};
use std::net::TcpListener;

use ed25519_dalek::{Signer, SigningKey};
use nc_launcher::update::{
    AvailableUpdate, OwnBuild, ReinstallDecision, UpdateError, apply_hook_update, check_for_update,
    download_verified, is_newer, parse_manifest, pick_files, reinstall_decision, restore_hook_dll,
    swap_in_hook_dll, verify_file, verify_manifest_signature, with_query,
};
use sha2::{Digest, Sha256};

use crate::scratch::fresh_dir;

const SIGNING_SEED: [u8; 32] = [7; 32];

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&SIGNING_SEED)
}

fn public_hex(key: &SigningKey) -> String {
    key.verifying_key()
        .to_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn sha_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn manifest_json(commit: &str, commit_time: u64, hook: &[u8], launcher: &[u8]) -> String {
    format!(
        r#"{{"channel":"test","commit":"{commit}","commit_time":{commit_time},"built_at":1,"files":[
{{"name":"replicant_hook.dll","kind":"hook","platform":"windows","size":{},"sha256":"{}"}},
{{"name":"nc_launcher.exe","kind":"launcher","platform":"windows","size":1,"sha256":"00"}},
{{"name":"nier-coop-launcher-linux","kind":"launcher","platform":"linux","size":{},"sha256":"{}"}}]}}"#,
        hook.len(),
        sha_hex(hook),
        launcher.len(),
        sha_hex(launcher)
    )
}

fn replicant_hook(manifest: &nc_launcher::update::Manifest) -> nc_launcher::update::ManifestFile {
    pick_files(manifest, "linux")
        .unwrap()
        .hook_for("replicant")
        .unwrap()
        .clone()
}

fn two_games_manifest_json() -> String {
    r#"{"channel":"test","commit":"c1","files":[
{"name":"alpha_hook.dll","kind":"hook","game":"alpha","platform":"windows","size":1,"sha256":"00"},
{"name":"beta_hook.dll","kind":"hook","game":"beta","platform":"windows","size":2,"sha256":"11"},
{"name":"nier-coop-launcher-linux","kind":"launcher","platform":"linux","size":1,"sha256":"00"}]}"#
        .to_owned()
}

fn own(commit: &str, commit_time: u64) -> OwnBuild {
    OwnBuild {
        commit: commit.to_owned(),
        commit_time,
    }
}

fn serve(routes: Vec<(&'static str, Vec<u8>)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    spawn_server(listener, routes);
    origin
}

fn spawn_server(listener: TcpListener, routes: Vec<(&'static str, Vec<u8>)>) {
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = [0u8; 2048];
            let read = stream.read(&mut request).unwrap_or(0);
            let text = String::from_utf8_lossy(&request[..read]).into_owned();
            let target = text.split_whitespace().nth(1).unwrap_or("");
            let path = target.split('?').next().unwrap_or("").to_owned();
            let found = routes.iter().find(|(route, _)| *route == path);
            let (status, body) = match found {
                Some((_, body)) => ("200 OK", body.clone()),
                None => ("404 Not Found", Vec::new()),
            };
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
}

#[test]
fn a_signed_manifest_is_accepted() {
    let key = signing_key();
    let bytes = b"manifest bytes";
    let signature = key.sign(bytes).to_bytes();
    assert!(verify_manifest_signature(bytes, &signature, &public_hex(&key)).is_ok());
}

#[test]
fn a_changed_manifest_is_rejected() {
    let key = signing_key();
    let signature = key.sign(b"manifest bytes").to_bytes();
    let result = verify_manifest_signature(b"manifest byteZ", &signature, &public_hex(&key));
    assert!(matches!(result, Err(UpdateError::BadSignature)));
}

#[test]
fn a_signature_from_another_key_is_rejected() {
    let other = SigningKey::from_bytes(&[9; 32]);
    let signature = other.sign(b"manifest bytes").to_bytes();
    let result =
        verify_manifest_signature(b"manifest bytes", &signature, &public_hex(&signing_key()));
    assert!(matches!(result, Err(UpdateError::BadSignature)));
}

#[test]
fn a_short_signature_or_bad_key_text_is_rejected() {
    let key = signing_key();
    assert!(verify_manifest_signature(b"x", &[0; 10], &public_hex(&key)).is_err());
    assert!(verify_manifest_signature(b"x", &[0; 64], "not hex").is_err());
}

#[test]
fn the_embedded_public_key_is_a_valid_ed25519_key() {
    let signature = [0u8; 64];
    let result =
        verify_manifest_signature(b"x", &signature, nc_launcher::update::UPDATE_PUBLIC_KEY_HEX);
    assert!(matches!(result, Err(UpdateError::BadSignature)));
    let bytes: Vec<u8> = (0..64)
        .step_by(2)
        .map(|at| {
            u8::from_str_radix(&nc_launcher::update::UPDATE_PUBLIC_KEY_HEX[at..at + 2], 16).unwrap()
        })
        .collect();
    assert!(ed25519_dalek::VerifyingKey::from_bytes(&bytes.try_into().unwrap()).is_ok());
}

#[test]
fn a_different_commit_that_is_newer_is_offered() {
    let manifest = parse_manifest(manifest_json("bbbbbbbbbb", 200, b"h", b"l").as_bytes()).unwrap();
    assert!(is_newer(&own("aaaaaaaaaa", 100), &manifest));
}

#[test]
fn the_same_commit_or_an_older_one_is_not_offered() {
    let manifest = parse_manifest(manifest_json("bbbbbbbbbb", 200, b"h", b"l").as_bytes()).unwrap();
    assert!(!is_newer(&own("bbbbbbbbbb", 200), &manifest));
    assert!(!is_newer(&own("cccccccccc", 300), &manifest));
}

#[test]
fn a_build_with_an_unknown_time_takes_any_different_commit() {
    let manifest = parse_manifest(manifest_json("bbbbbbbbbb", 200, b"h", b"l").as_bytes()).unwrap();
    assert!(is_newer(&own("unknown", 0), &manifest));
}

#[test]
fn the_platform_pick_takes_that_platforms_launcher_and_the_windows_hook() {
    let manifest = parse_manifest(manifest_json("b", 1, b"h", b"l").as_bytes()).unwrap();
    let linux = pick_files(&manifest, "linux").unwrap();
    assert_eq!(linux.launcher.name, "nier-coop-launcher-linux");
    assert_eq!(
        linux.hook_for("replicant").unwrap().name,
        "replicant_hook.dll"
    );
    let windows = pick_files(&manifest, "windows").unwrap();
    assert_eq!(windows.launcher.name, "nc_launcher.exe");
    assert!(matches!(
        pick_files(&manifest, "macos"),
        Err(UpdateError::MissingFile(_))
    ));
}

#[test]
fn a_manifest_with_a_path_in_a_file_name_is_rejected() {
    let text =
        manifest_json("b", 1, b"h", b"l").replace("replicant_hook.dll", "../replicant_hook.dll");
    assert!(matches!(
        parse_manifest(text.as_bytes()),
        Err(UpdateError::BadManifest(_))
    ));
    assert!(parse_manifest(b"not json").is_err());
}

#[test]
fn a_file_with_the_wrong_hash_or_size_is_rejected() {
    let dir = fresh_dir("update-verify");
    let path = dir.join("file.bin");
    std::fs::write(&path, b"hello").unwrap();
    let manifest = parse_manifest(manifest_json("b", 1, b"hello", b"hello").as_bytes()).unwrap();
    let good = replicant_hook(&manifest);
    assert!(verify_file(&path, &good).is_ok());
    let mut wrong_hash = good.clone();
    wrong_hash.sha256 = sha_hex(b"other");
    assert!(matches!(
        verify_file(&path, &wrong_hash),
        Err(UpdateError::FileMismatch(_))
    ));
    let mut wrong_size = good;
    wrong_size.size += 1;
    assert!(verify_file(&path, &wrong_size).is_err());
}

#[test]
fn a_download_whose_hash_differs_is_deleted_and_rejected() {
    let dir = fresh_dir("update-download");
    let base = serve(vec![("/replicant_hook.dll", b"tampered".to_vec())]);
    let manifest = parse_manifest(manifest_json("b", 1, b"genuine", b"l").as_bytes()).unwrap();
    let hook = replicant_hook(&manifest);
    let destination = dir.join("replicant_hook.dll");
    assert!(download_verified(&format!("{base}/replicant_hook.dll"), &hook, &destination).is_err());
    assert!(!destination.exists());
}

#[test]
fn a_matching_download_is_kept() {
    let dir = fresh_dir("update-download-ok");
    let base = serve(vec![("/replicant_hook.dll", b"genuine".to_vec())]);
    let manifest = parse_manifest(manifest_json("b", 1, b"genuine", b"l").as_bytes()).unwrap();
    let hook = replicant_hook(&manifest);
    let destination = dir.join("replicant_hook.dll");
    download_verified(&format!("{base}/replicant_hook.dll"), &hook, &destination).unwrap();
    assert_eq!(std::fs::read(destination).unwrap(), b"genuine");
}

fn check_against(
    manifest: String,
    signing: &SigningKey,
    own_build: &OwnBuild,
) -> Result<Option<AvailableUpdate>, UpdateError> {
    let signature = signing.sign(manifest.as_bytes()).to_bytes().to_vec();
    let origin = serve(vec![
        ("/test/manifest.json", manifest.into_bytes()),
        ("/test/manifest.json.sig", signature),
    ]);
    check_for_update(
        &format!("{origin}/test/manifest.json"),
        &public_hex(&signing_key()),
        own_build,
        "linux",
    )
}

#[test]
fn a_check_finds_a_newer_signed_build() {
    let found = check_against(
        manifest_json("bbbbbbbbbb", 200, b"h", b"l"),
        &signing_key(),
        &own("aaaaaaaaaa", 100),
    );
    let update = found.unwrap().unwrap();
    assert_eq!(update.manifest.commit, "bbbbbbbbbb");
    assert_eq!(update.plan.launcher.name, "nier-coop-launcher-linux");
}

#[test]
fn a_check_on_the_same_commit_finds_nothing() {
    let found = check_against(
        manifest_json("aaaaaaaaaa", 100, b"h", b"l"),
        &signing_key(),
        &own("aaaaaaaaaa", 100),
    );
    assert!(found.unwrap().is_none());
}

#[test]
fn a_check_with_a_foreign_signature_fails() {
    let other = SigningKey::from_bytes(&[9; 32]);
    let found = check_against(
        manifest_json("bbbbbbbbbb", 200, b"h", b"l"),
        &other,
        &own("aaaaaaaaaa", 100),
    );
    assert!(matches!(found, Err(UpdateError::BadSignature)));
}

#[test]
fn a_refused_request_gives_one_refused_line() {
    let base = serve(Vec::new());
    let result = check_for_update(
        &format!("{base}/manifest.json"),
        &public_hex(&signing_key()),
        &own("a", 1),
        "linux",
    );
    assert_eq!(
        result.unwrap_err().to_string(),
        "update server refused the request (404)"
    );
}

#[test]
fn files_are_fetched_from_the_manifests_own_folder() {
    let found = check_against(
        manifest_json("bbbbbbbbbb", 200, b"h", b"l"),
        &signing_key(),
        &own("aaaaaaaaaa", 100),
    );
    let update = found.unwrap().unwrap();
    assert!(update.base_url.ends_with("/test/"));
    assert!(
        update
            .file_url("replicant_hook.dll")
            .ends_with("/test/replicant_hook.dll?build=bbbbbbbbbb")
    );
}

#[test]
fn a_query_is_added_with_a_question_mark_or_an_ampersand() {
    assert_eq!(with_query("http://h/a", "k", "v"), "http://h/a?k=v");
    assert_eq!(with_query("http://h/a?x=1", "k", "v"), "http://h/a?x=1&k=v");
}

#[test]
fn a_stale_first_copy_is_retried_with_a_unique_query_and_then_kept() {
    let hook = b"new hook".to_vec();
    let manifest = parse_manifest(manifest_json("b", 1, &hook, b"l").as_bytes()).unwrap();
    let expected = replicant_hook(&manifest);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = [0u8; 2048];
            let read = stream.read(&mut request).unwrap_or(0);
            let text = String::from_utf8_lossy(&request[..read]).into_owned();
            let target = text.split_whitespace().nth(1).unwrap_or("").to_owned();
            let body: &[u8] = if target.contains("retry=") {
                b"new hook"
            } else {
                b"old hook"
            };
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(body);
        }
    });
    let destination = fresh_dir("stale-retry").join("replicant_hook.dll");
    download_verified(
        &format!("{base}/replicant_hook.dll?build=abc"),
        &expected,
        &destination,
    )
    .unwrap();
    assert_eq!(std::fs::read(&destination).unwrap(), hook);
}

#[test]
fn a_copy_that_stays_wrong_after_the_retry_says_the_server_sent_an_old_copy() {
    let hook = b"new hook".to_vec();
    let manifest = parse_manifest(manifest_json("b", 1, &hook, b"l").as_bytes()).unwrap();
    let expected = replicant_hook(&manifest);
    let base = serve(vec![("/replicant_hook.dll", b"old hook".to_vec())]);
    let destination = fresh_dir("stale-fail").join("replicant_hook.dll");
    let error = download_verified(
        &format!("{base}/replicant_hook.dll"),
        &expected,
        &destination,
    )
    .unwrap_err();
    assert!(matches!(error, UpdateError::StaleCopy(_)));
    assert!(error.to_string().contains("old copy"));
    assert!(!destination.exists());
}

#[test]
fn swapping_the_hook_dll_keeps_a_backup_that_restore_puts_back() {
    let dir = fresh_dir("update-swap");
    let destination = dir.join("replicant_hook.dll");
    let staged = dir.join("staged.dll");
    std::fs::write(&destination, b"old").unwrap();
    std::fs::write(&staged, b"new").unwrap();
    let backup = swap_in_hook_dll(&staged, &destination).unwrap().unwrap();
    assert_eq!(std::fs::read(&destination).unwrap(), b"new");
    restore_hook_dll(&backup, &destination);
    assert_eq!(std::fs::read(&destination).unwrap(), b"old");
}

#[test]
fn a_hook_only_update_replaces_the_dll_and_leaves_the_launcher() {
    let manifest = manifest_json("bbbbbbbbbb", 200, b"new hook", b"new launcher");
    let signature = signing_key().sign(manifest.as_bytes()).to_bytes().to_vec();
    let origin = serve(vec![
        ("/test/manifest.json", manifest.into_bytes()),
        ("/test/manifest.json.sig", signature),
        ("/test/replicant_hook.dll", b"new hook".to_vec()),
    ]);
    let update = check_for_update(
        &format!("{origin}/test/manifest.json"),
        &public_hex(&signing_key()),
        &own("aaaaaaaaaa", 100),
        "linux",
    )
    .unwrap()
    .unwrap();
    let dir = fresh_dir("update-hook-only");
    let exe = dir.join("nc_launcher");
    std::fs::write(&exe, b"old launcher").unwrap();
    std::fs::write(dir.join("replicant_hook.dll"), b"old hook").unwrap();
    apply_hook_update(&update, &exe).unwrap();
    assert_eq!(
        std::fs::read(dir.join("replicant_hook.dll")).unwrap(),
        b"new hook"
    );
    assert_eq!(std::fs::read(&exe).unwrap(), b"old launcher");
}

#[test]
fn a_manifest_with_two_games_hooks_gives_each_game_its_own_file() {
    let manifest = parse_manifest(two_games_manifest_json().as_bytes()).unwrap();
    let plan = pick_files(&manifest, "linux").unwrap();
    assert_eq!(plan.hooks.len(), 2);
    assert_eq!(plan.hook_for("alpha").unwrap().name, "alpha_hook.dll");
    assert_eq!(plan.hook_for("beta").unwrap().name, "beta_hook.dll");
    assert!(plan.hook_for("replicant").is_none());
}

#[test]
fn a_hook_entry_with_no_game_is_the_replicant_hook() {
    let manifest = parse_manifest(manifest_json("b", 1, b"h", b"l").as_bytes()).unwrap();
    let plan = pick_files(&manifest, "linux").unwrap();
    assert_eq!(plan.hooks.len(), 1);
    assert_eq!(plan.hooks[0].game_id, "replicant");
    assert_eq!(
        plan.hook_for("replicant").unwrap().name,
        "replicant_hook.dll"
    );
}

#[test]
fn a_manifest_with_no_hook_at_all_is_refused() {
    let text = r#"{"channel":"test","commit":"c1","files":[
{"name":"nier-coop-launcher-linux","kind":"launcher","platform":"linux","size":1,"sha256":"00"}]}"#;
    let manifest = parse_manifest(text.as_bytes()).unwrap();
    assert!(matches!(
        pick_files(&manifest, "linux"),
        Err(UpdateError::MissingFile(_))
    ));
}

#[test]
fn a_hook_update_skips_a_game_the_launcher_does_not_know() {
    let manifest = format!(
        r#"{{"channel":"test","commit":"bbbbbbbbbb","commit_time":200,"files":[
{{"name":"replicant_hook.dll","kind":"hook","platform":"windows","size":8,"sha256":"{}"}},
{{"name":"alpha_hook.dll","kind":"hook","game":"alpha","platform":"windows","size":3,"sha256":"{}"}},
{{"name":"nier-coop-launcher-linux","kind":"launcher","platform":"linux","size":1,"sha256":"00"}}]}}"#,
        sha_hex(b"new hook"),
        sha_hex(b"abc"),
    );
    let signature = signing_key().sign(manifest.as_bytes()).to_bytes().to_vec();
    let origin = serve(vec![
        ("/test/manifest.json", manifest.into_bytes()),
        ("/test/manifest.json.sig", signature),
        ("/test/replicant_hook.dll", b"new hook".to_vec()),
    ]);
    let update = check_for_update(
        &format!("{origin}/test/manifest.json"),
        &public_hex(&signing_key()),
        &own("aaaaaaaaaa", 100),
        "linux",
    )
    .unwrap()
    .unwrap();
    let dir = fresh_dir("update-skip-unknown-game");
    let exe = dir.join("nc_launcher");
    std::fs::write(&exe, b"old launcher").unwrap();
    std::fs::write(dir.join("replicant_hook.dll"), b"old hook").unwrap();
    apply_hook_update(&update, &exe).unwrap();
    assert_eq!(
        std::fs::read(dir.join("replicant_hook.dll")).unwrap(),
        b"new hook"
    );
    assert!(!dir.join("alpha_hook.dll").exists());
}

#[test]
fn a_manifest_with_both_games_reinstalls_each_games_hook() {
    let json = r#"{"channel":"test","commit":"c1","files":[
{"name":"replicant_hook.dll","kind":"hook","game":"replicant","platform":"windows","size":1,"sha256":"00"},
{"name":"automata_hook.dll","kind":"hook","game":"automata","platform":"windows","size":2,"sha256":"11"},
{"name":"nier-coop-launcher-linux","kind":"launcher","platform":"linux","size":1,"sha256":"00"}]}"#;
    let plan = pick_files(&parse_manifest(json.as_bytes()).unwrap(), "linux").unwrap();
    assert_eq!(plan.hooks.len(), 2);
    assert_eq!(plan.hook_for("automata").unwrap().name, "automata_hook.dll");
    assert_eq!(
        reinstall_decision(&plan, &nc_launcher_core::AUTOMATA),
        ReinstallDecision::InstallFromThisUpdate
    );
    assert_eq!(
        reinstall_decision(&plan, &nc_launcher_core::REPLICANT),
        ReinstallDecision::InstallFromThisUpdate
    );
}

#[test]
fn a_manifest_with_only_the_replicant_hook_leaves_the_second_game_alone_and_says_so() {
    let manifest = parse_manifest(manifest_json("b", 1, b"h", b"l").as_bytes()).unwrap();
    let plan = pick_files(&manifest, "linux").unwrap();
    assert_eq!(
        reinstall_decision(&plan, &nc_launcher_core::REPLICANT),
        ReinstallDecision::InstallFromThisUpdate
    );
    assert_eq!(
        reinstall_decision(&plan, &nc_launcher_core::AUTOMATA),
        ReinstallDecision::ManifestHasNoHookForThisGame
    );
    assert_eq!(
        nc_launcher::update::untouched_install_line(&nc_launcher_core::AUTOMATA),
        "this build has no hook for NieR:Automata; its installed mod was left as it is"
    );
}
