use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::Replicant;
use rc_game::player_name::local_name;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::AutoConfig;

static LOGGED: AtomicBool = AtomicBool::new(false);
static LOCAL_NAME: Mutex<Option<String>> = Mutex::new(None);

pub fn log_local_name_once(build: &VerifiedBuild) {
    if LOGGED.swap(true, Ordering::Relaxed) {
        return;
    }
    let configured = AutoConfig::load_beside_game()
        .ok()
        .flatten()
        .and_then(|config| config.name);
    let save_block = Replicant::new(InProcessMemory::new(build))
        .save_name()
        .unwrap_or_default();
    let name = local_name(configured.as_deref(), &save_block);
    *LOCAL_NAME.lock().unwrap_or_else(PoisonError::into_inner) = name.clone();
    match name {
        Some(name) if configured.as_deref().is_some_and(|set| set.trim() == name) => {
            log_line(&format!(
                "names: local player is {name} (from the launcher)"
            ));
        }
        Some(name) => log_line(&format!("names: local player is {name} (from the save)")),
        None => log_line("names: the local player has no name"),
    }
}

pub fn local_player_name() -> Option<String> {
    LOCAL_NAME
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}
