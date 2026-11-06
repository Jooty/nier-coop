#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IrohConfig {
    pub use_relays_and_lookup: bool,
}

impl Default for IrohConfig {
    fn default() -> Self {
        Self {
            use_relays_and_lookup: true,
        }
    }
}
