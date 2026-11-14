use crate::env_names::{old_env_name, old_name_note};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvRead<T> {
    pub value: Option<T>,
    pub old_name_used: Option<String>,
}

pub fn read_env_with_old_name<T>(name: &str, get: impl Fn(&str) -> Option<T>) -> EnvRead<T> {
    if let Some(value) = get(name) {
        return EnvRead {
            value: Some(value),
            old_name_used: None,
        };
    }
    let Some(old_name) = old_env_name(name) else {
        return EnvRead {
            value: None,
            old_name_used: None,
        };
    };
    match get(&old_name) {
        Some(value) => EnvRead {
            value: Some(value),
            old_name_used: Some(old_name),
        },
        None => EnvRead {
            value: None,
            old_name_used: None,
        },
    }
}

pub fn read_env_noting_old_name(name: &str, note: impl FnOnce(&str)) -> Option<String> {
    let read = read_env_with_old_name(name, |key| std::env::var(key).ok());
    if let Some(old_name) = &read.old_name_used {
        note(&old_name_note(old_name, name));
    }
    read.value
}

pub fn read_env_os_noting_old_name(
    name: &str,
    note: impl FnOnce(&str),
) -> Option<std::ffi::OsString> {
    let read = read_env_with_old_name(name, |key| std::env::var_os(key));
    if let Some(old_name) = &read.old_name_used {
        note(&old_name_note(old_name, name));
    }
    read.value
}
