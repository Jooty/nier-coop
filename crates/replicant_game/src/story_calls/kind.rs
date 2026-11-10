#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryCallKind {
    Local,
    Story,
    Container,
}

impl StoryCallKind {
    pub fn is_relayed(self) -> bool {
        self == Self::Story
    }
}
