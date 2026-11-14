mod exe_facts;
mod exe_identity;
mod exe_variant;
mod known_exes;

pub use exe_facts::ExeFacts;
pub use exe_identity::ExeIdentity;
pub use exe_variant::{ExeVariant, exe_difference_line};
pub use known_exes::{COMPAT_EXE, KNOWN_EXES, NEW_MAIN_EXE, OLD_MAIN_EXE, identify};
