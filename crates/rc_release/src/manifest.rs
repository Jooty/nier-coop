mod document;
mod entry;

pub use document::{Manifest, ManifestInputs, build_manifest, write_manifest};
pub use entry::{FileKind, FileSpec, ManifestFile, describe_file, parse_file_spec};
