#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerSlot(pub u8);

impl PlayerSlot {
    pub const HOST: Self = Self(0);

    pub fn is_host(self) -> bool {
        self == Self::HOST
    }
}
