mod manifest_file;

pub use manifest_file::{sign_bytes, sign_manifest_file};

pub const SIGNATURE_SUFFIX: &str = ".sig";
