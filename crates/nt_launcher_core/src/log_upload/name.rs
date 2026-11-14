use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::log_upload::{INSTALL_ID_LENGTH, MAX_UPLOAD_NAME_STEM_LENGTH, UPLOAD_NAME_SUFFIX};

pub fn new_install_id() -> String {
    format!("{:016x}{:016x}", random_word(), random_word())
}

pub fn is_valid_install_id(id: &str) -> bool {
    id.len() == INSTALL_ID_LENGTH && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn upload_file_name(stamp: &str, role: &str) -> Option<String> {
    let name = format!("{stamp}-{role}{UPLOAD_NAME_SUFFIX}");
    is_allowed_upload_name(&name).then_some(name)
}

pub fn is_allowed_upload_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(UPLOAD_NAME_SUFFIX) else {
        return false;
    };
    !stem.is_empty()
        && stem.len() <= MAX_UPLOAD_NAME_STEM_LENGTH
        && stem
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn random_word() -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    hasher.write_u128(nanos);
    hasher.write_u32(std::process::id());
    hasher.finish()
}
