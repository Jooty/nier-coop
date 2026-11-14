use crate::exe::ExeVariant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameAddress {
    pub main: u32,
    pub compat: u32,
}

impl GameAddress {
    pub const fn new(main: u32, compat: u32) -> Self {
        Self { main, compat }
    }

    pub const fn rva(self, variant: ExeVariant) -> u32 {
        match variant {
            ExeVariant::Main => self.main,
            ExeVariant::Compat => self.compat,
        }
    }
}
