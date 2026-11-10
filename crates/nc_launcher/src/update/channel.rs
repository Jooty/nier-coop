use std::sync::OnceLock;

use crate::update::{
    BUILD_COMMIT, CHANNEL_FROM_BUILD, DEV_LOG_TOKEN_FROM_BUILD, DEV_UPDATE_URL_FROM_BUILD,
    RELEASE_CHANNEL_NAME, RELEASE_PUBLIC_KEY_HEX_FROM_BUILD, RELEASE_UPDATE_URL,
    UPDATE_PUBLIC_KEY_HEX, VERSION_FROM_BUILD, short_commit,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Dev,
    Release,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BuildInputs<'a> {
    pub channel: Option<&'a str>,
    pub version: Option<&'a str>,
    pub dev_update_url: Option<&'a str>,
    pub dev_log_token: Option<&'a str>,
    pub release_public_key_hex: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildConfig {
    pub channel: Channel,
    pub version: String,
    pub update_url: Option<String>,
    pub log_token: Option<String>,
    pub public_key_hex: String,
}

pub fn resolve_build_config(inputs: BuildInputs, dev_public_key_hex: &str) -> BuildConfig {
    let version = inputs.version.map(str::trim).unwrap_or_default().to_owned();
    if inputs.channel.map(str::trim) == Some(RELEASE_CHANNEL_NAME) {
        return BuildConfig {
            channel: Channel::Release,
            version,
            update_url: Some(RELEASE_UPDATE_URL.to_owned()),
            log_token: None,
            public_key_hex: inputs
                .release_public_key_hex
                .map(str::trim)
                .unwrap_or_default()
                .to_owned(),
        };
    }
    BuildConfig {
        channel: Channel::Dev,
        version,
        update_url: inputs.dev_update_url.map(str::to_owned),
        log_token: inputs.dev_log_token.map(str::to_owned),
        public_key_hex: dev_public_key_hex.to_owned(),
    }
}

pub fn build_config() -> &'static BuildConfig {
    static CONFIG: OnceLock<BuildConfig> = OnceLock::new();
    CONFIG.get_or_init(|| {
        resolve_build_config(
            BuildInputs {
                channel: CHANNEL_FROM_BUILD,
                version: VERSION_FROM_BUILD,
                dev_update_url: DEV_UPDATE_URL_FROM_BUILD,
                dev_log_token: DEV_LOG_TOKEN_FROM_BUILD,
                release_public_key_hex: RELEASE_PUBLIC_KEY_HEX_FROM_BUILD,
            },
            UPDATE_PUBLIC_KEY_HEX,
        )
    })
}

pub fn channel_label(config: &BuildConfig, commit: &str) -> String {
    match config.channel {
        Channel::Release if config.version.is_empty() => RELEASE_CHANNEL_NAME.to_owned(),
        Channel::Release => format!("{RELEASE_CHANNEL_NAME} {}", config.version),
        Channel::Dev => format!("dev {}", short_commit(commit)),
    }
}

pub fn own_channel_label() -> String {
    channel_label(build_config(), BUILD_COMMIT)
}
