/// Which of the two ways into a lobby the entry menu's pointer is on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EntryChoice {
    #[default]
    Host,
    Join,
}
