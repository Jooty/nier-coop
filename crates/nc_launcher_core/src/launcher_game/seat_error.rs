#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatError {
    FileSizeWrong,
    SlotSizeWrong,
    NoSuchSlot,
    NotAGameSlot,
    EmptySlot,
    ChecksumWrong,
    GameKeepsNoSaves,
}
