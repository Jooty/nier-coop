#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetCommand {
    Host,
    Join(String),
    Leave,
}
