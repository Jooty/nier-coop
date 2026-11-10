use std::fs;

use nc_launcher::update::{FileKind, parse_manifest, pick_files, verify_manifest_signature};
use nc_release::cli::run;
use nc_release::key::decode_hex;
use nc_release::manifest::parse_file_spec;

use crate::scratch::fresh_dir;

const HOOK_BYTES: &[u8] = b"hook dll bytes";
const LAUNCHER_BYTES: &[u8] = b"linux launcher bytes";

fn arguments(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

#[test]
fn keygen_prints_only_the_public_key_and_writes_a_private_seed() {
    let dir = fresh_dir("keygen");
    let key_file = dir.join("release.key");
    let printed = run(&arguments(&["keygen", "--out", key_file.to_str().unwrap()])).unwrap();
    let seed_text = fs::read_to_string(&key_file).unwrap();
    assert_eq!(printed.len(), 64);
    assert!(!printed.contains(seed_text.trim()));
    assert_eq!(decode_hex(seed_text.trim()).unwrap().len(), 32);
}

#[cfg(unix)]
#[test]
fn the_private_key_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let dir = fresh_dir("key_mode");
    let key_file = dir.join("release.key");
    run(&arguments(&["keygen", "--out", key_file.to_str().unwrap()])).unwrap();
    let mode = fs::metadata(&key_file).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn keygen_refuses_to_overwrite_an_existing_key() {
    let dir = fresh_dir("keygen_twice");
    let key_file = dir.join("release.key");
    let args = arguments(&["keygen", "--out", key_file.to_str().unwrap()]);
    run(&args).unwrap();
    let before = fs::read_to_string(&key_file).unwrap();
    assert!(run(&args).is_err());
    assert_eq!(fs::read_to_string(&key_file).unwrap(), before);
}

#[test]
fn the_manifest_parses_with_the_launcher_and_picks_both_files() {
    let dir = fresh_dir("manifest");
    let hook = dir.join("replicant_hook.dll");
    let launcher = dir.join("nier-coop-launcher-linux");
    fs::write(&hook, HOOK_BYTES).unwrap();
    fs::write(&launcher, LAUNCHER_BYTES).unwrap();
    let out = dir.join("manifest.json");
    run(&arguments(&[
        "manifest",
        "--channel",
        "release",
        "--commit",
        "0123456789abcdef",
        "--commit-time",
        "1700000000",
        "--file",
        &format!("{}:hook:windows", hook.display()),
        "--file",
        &format!("{}:launcher:linux", launcher.display()),
        "--out",
        out.to_str().unwrap(),
    ]))
    .unwrap();
    let manifest = parse_manifest(&fs::read(&out).unwrap()).unwrap();
    assert_eq!(manifest.channel, "release");
    assert_eq!(manifest.commit, "0123456789abcdef");
    assert_eq!(manifest.commit_time, 1_700_000_000);
    let plan = pick_files(&manifest, "linux").unwrap();
    assert_eq!(plan.hook.name, "replicant_hook.dll");
    assert_eq!(plan.hook.kind, FileKind::Hook);
    assert_eq!(plan.hook.size, HOOK_BYTES.len() as u64);
    assert_eq!(plan.launcher.name, "nier-coop-launcher-linux");
    assert_eq!(plan.launcher.size, LAUNCHER_BYTES.len() as u64);
}

#[test]
fn a_signature_from_the_tool_verifies_with_the_launchers_verify_fn() {
    let dir = fresh_dir("sign");
    let key_file = dir.join("release.key");
    let public_hex = run(&arguments(&["keygen", "--out", key_file.to_str().unwrap()])).unwrap();
    let manifest = dir.join("manifest.json");
    fs::write(
        &manifest,
        br#"{"channel":"release","commit":"abc","files":[]}"#,
    )
    .unwrap();
    run(&arguments(&[
        "sign",
        "--key-file",
        key_file.to_str().unwrap(),
        manifest.to_str().unwrap(),
    ]))
    .unwrap();
    let signature = fs::read(dir.join("manifest.json.sig")).unwrap();
    let manifest_bytes = fs::read(&manifest).unwrap();
    verify_manifest_signature(&manifest_bytes, &signature, &public_hex).unwrap();
    assert!(verify_manifest_signature(b"tampered", &signature, &public_hex).is_err());
}

#[test]
fn a_file_spec_keeps_a_colon_in_the_path() {
    let spec = parse_file_spec("C:\\build\\replicant_hook.dll:hook:windows").unwrap();
    assert_eq!(spec.path.to_str().unwrap(), "C:\\build\\replicant_hook.dll");
    assert_eq!(spec.platform, "windows");
    assert!(parse_file_spec("replicant_hook.dll:plugin:windows").is_err());
    assert!(parse_file_spec("replicant_hook.dll:hook:").is_err());
}
