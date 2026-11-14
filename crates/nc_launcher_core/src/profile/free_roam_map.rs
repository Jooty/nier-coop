/// A place free roam can start at: the game's own name for it and the hook's start candidate that lands there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreeRoamMap {
    pub id: &'static str,
    pub label: &'static str,
    pub start_index: u8,
}
