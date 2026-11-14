use nt_launcher::update::{
    BuildInputs, Channel, RELEASE_UPDATE_URL, channel_label, resolve_build_config,
    verify_manifest_signature,
};
use nt_launcher_core::log_upload_offered;

const DEV_KEY: &str = "dev-key";
const DEV_URL: &str = "https://dev.invalid/manifest.json";
const RELEASE_KEY: &str = "release-key";

fn release_inputs() -> BuildInputs<'static> {
    BuildInputs {
        channel: Some("release"),
        version: Some("v0.2.0"),
        dev_update_url: Some(DEV_URL),
        dev_log_token: Some("token"),
        release_public_key_hex: Some(RELEASE_KEY),
    }
}

#[test]
fn a_build_without_a_channel_is_a_dev_build_with_the_dev_settings() {
    let config = resolve_build_config(
        BuildInputs {
            dev_update_url: Some(DEV_URL),
            dev_log_token: Some("token"),
            release_public_key_hex: Some(RELEASE_KEY),
            ..BuildInputs::default()
        },
        DEV_KEY,
    );
    assert_eq!(config.channel, Channel::Dev);
    assert_eq!(config.update_url.as_deref(), Some(DEV_URL));
    assert_eq!(config.log_token.as_deref(), Some("token"));
    assert_eq!(config.public_key_hex, DEV_KEY);
}

#[test]
fn a_release_build_reads_the_github_url_and_the_release_key() {
    let config = resolve_build_config(release_inputs(), DEV_KEY);
    assert_eq!(config.channel, Channel::Release);
    assert_eq!(config.update_url.as_deref(), Some(RELEASE_UPDATE_URL));
    assert_eq!(config.public_key_hex, RELEASE_KEY);
    assert_eq!(config.version, "v0.2.0");
}

#[test]
fn a_release_build_has_no_log_token_so_no_log_upload() {
    let config = resolve_build_config(release_inputs(), DEV_KEY);
    assert_eq!(config.log_token, None);
    assert!(!log_upload_offered(
        config.update_url.as_deref(),
        config.log_token.as_deref()
    ));
}

#[test]
fn a_release_build_without_a_release_key_trusts_nothing() {
    let config = resolve_build_config(
        BuildInputs {
            release_public_key_hex: None,
            ..release_inputs()
        },
        DEV_KEY,
    );
    assert_eq!(config.public_key_hex, "");
    assert!(verify_manifest_signature(b"x", &[0; 64], &config.public_key_hex).is_err());
}

#[test]
fn the_label_names_the_channel_and_the_version_or_the_commit() {
    let release = resolve_build_config(release_inputs(), DEV_KEY);
    assert_eq!(
        channel_label(&release, "0123456789abcdef"),
        "release v0.2.0"
    );
    let dev = resolve_build_config(BuildInputs::default(), DEV_KEY);
    assert_eq!(channel_label(&dev, "0123456789abcdef"), "dev 0123456");
}
