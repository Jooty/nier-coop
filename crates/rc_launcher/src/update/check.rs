use crate::update::fetch::{base_url_of, http_get};
use crate::update::files::with_query;
use crate::update::{
    MANIFEST_SIGNATURE_SUFFIX, MAX_MANIFEST_BYTES, Manifest, OwnBuild, UpdateError, UpdatePlan,
    is_newer, parse_manifest, pick_files, verify_manifest_signature,
};

const SIGNATURE_MAX_BYTES: u64 = 256;
const BUILD_QUERY_KEY: &str = "build";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub manifest: Manifest,
    pub plan: UpdatePlan,
    pub base_url: String,
}

impl AvailableUpdate {
    pub fn file_url(&self, name: &str) -> String {
        with_query(
            &format!("{}{name}", self.base_url),
            BUILD_QUERY_KEY,
            &self.manifest.commit,
        )
    }
}

pub fn check_for_update(
    manifest_url: &str,
    public_key_hex: &str,
    own: &OwnBuild,
    platform: &str,
) -> Result<Option<AvailableUpdate>, UpdateError> {
    let manifest_bytes = http_get(manifest_url, MAX_MANIFEST_BYTES)?;
    let signature_url = format!("{manifest_url}{MANIFEST_SIGNATURE_SUFFIX}");
    let signature_bytes = http_get(&signature_url, SIGNATURE_MAX_BYTES)?;
    verify_manifest_signature(&manifest_bytes, &signature_bytes, public_key_hex)?;
    let manifest = parse_manifest(&manifest_bytes)?;
    if !is_newer(own, &manifest) {
        return Ok(None);
    }
    let plan = pick_files(&manifest, platform)?;
    Ok(Some(AvailableUpdate {
        manifest,
        plan,
        base_url: base_url_of(manifest_url).to_owned(),
    }))
}
