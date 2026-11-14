use crate::env_names::{ENV_PREFIX, OLD_ENV_PREFIX};

pub fn old_env_name(name: &str) -> Option<String> {
    name.strip_prefix(ENV_PREFIX)
        .map(|rest| format!("{OLD_ENV_PREFIX}{rest}"))
}

pub fn old_name_note(old_name: &str, name: &str) -> String {
    format!(
        "env: {old_name} is the old name of {name}; the old name still works, set {name} instead"
    )
}
