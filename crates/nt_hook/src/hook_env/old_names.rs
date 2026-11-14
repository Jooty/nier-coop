use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use nt_wire::{old_name_note, read_env_with_old_name};

use crate::diagnostics::log_line;

static OLD_NAMES_NOT_YET_LOGGED: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());

pub fn read_env(name: &str) -> Option<String> {
    let read = read_env_with_old_name(name, |key| std::env::var(key).ok());
    if let Some(old_name) = read.old_name_used {
        OLD_NAMES_NOT_YET_LOGGED
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(name.to_owned(), old_name);
    }
    read.value
}

pub fn log_old_env_names_in_use() {
    let pending = std::mem::take(
        &mut *OLD_NAMES_NOT_YET_LOGGED
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
    );
    for (name, old_name) in pending {
        log_line(&old_name_note(&old_name, &name));
    }
}
