#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaceRelation {
    Same,
    Other,
    #[default]
    Unknown,
}
