#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExeVariant {
    Main,
    Compat,
}

impl ExeVariant {
    pub const ALL: [Self; 2] = [Self::Main, Self::Compat];

    pub fn label(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Compat => "compat",
        }
    }
}
