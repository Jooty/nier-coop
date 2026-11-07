const SAVE_MANAGER_IDLE: i32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadProgress {
    Request,
    Wait,
    Loaded,
    Failed(u8),
}

pub fn reload_progress(requested: bool, manager_state: i32, result: u8) -> ReloadProgress {
    match (requested, manager_state) {
        (false, SAVE_MANAGER_IDLE) => ReloadProgress::Request,
        (true, SAVE_MANAGER_IDLE) if result == 0 => ReloadProgress::Loaded,
        (true, SAVE_MANAGER_IDLE) => ReloadProgress::Failed(result),
        _ => ReloadProgress::Wait,
    }
}
