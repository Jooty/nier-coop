use crate::log_upload::{UPLOAD_PATH_PREFIX, is_allowed_upload_name, is_valid_install_id};

pub fn origin_of(url: &str) -> Option<&str> {
    let (scheme, rest) = url.split_once("://")?;
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    if host_end == 0 {
        return None;
    }
    Some(&url[..scheme.len() + 3 + host_end])
}

pub fn upload_url(update_url: &str, install_id: &str, file_name: &str) -> Option<String> {
    if !is_valid_install_id(install_id) || !is_allowed_upload_name(file_name) {
        return None;
    }
    let origin = origin_of(update_url)?;
    Some(format!(
        "{origin}/{UPLOAD_PATH_PREFIX}/{install_id}/{file_name}"
    ))
}

pub fn log_upload_offered(update_url: Option<&str>, token: Option<&str>) -> bool {
    let present = |value: Option<&str>| value.is_some_and(|text| !text.trim().is_empty());
    present(update_url) && present(token)
}
