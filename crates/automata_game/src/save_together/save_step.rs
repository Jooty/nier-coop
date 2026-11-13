#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SaveStep {
    pub tell: Vec<u32>,
    pub request_save: bool,
    pub logs: Vec<String>,
}
