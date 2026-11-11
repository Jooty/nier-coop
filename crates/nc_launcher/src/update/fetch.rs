use std::time::Duration;

use ureq::Agent;

use crate::update::{HTTP_TIMEOUT_SECONDS, UpdateError};

pub fn base_url_of(manifest_url: &str) -> &str {
    manifest_url
        .rfind('/')
        .map_or(manifest_url, |slash| &manifest_url[..=slash])
}

pub fn http_get(url: &str, max_bytes: u64) -> Result<Vec<u8>, UpdateError> {
    get_bytes(http_agent().get(url), max_bytes)
}

fn get_bytes(
    request: ureq::RequestBuilder<ureq::typestate::WithoutBody>,
    max_bytes: u64,
) -> Result<Vec<u8>, UpdateError> {
    let response = request.call().map_err(describe_request_error)?;
    response
        .into_body()
        .with_config()
        .limit(max_bytes)
        .read_to_vec()
        .map_err(|error| UpdateError::Network(error.to_string()))
}

pub(super) fn open_download(url: &str) -> Result<impl std::io::Read + use<>, UpdateError> {
    let response = http_agent()
        .get(url)
        .call()
        .map_err(describe_request_error)?;
    Ok(response.into_body().into_reader())
}

fn http_agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(HTTP_TIMEOUT_SECONDS)))
        .user_agent(format!(
            "nier-coop-launcher/{} ({})",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS
        ))
        .build()
        .into()
}

fn describe_request_error(error: ureq::Error) -> UpdateError {
    match error {
        ureq::Error::StatusCode(code) => UpdateError::ServerRefused(code),
        other => UpdateError::Network(other.to_string()),
    }
}
