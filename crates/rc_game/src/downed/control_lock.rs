pub const DOWNED_CONTROL_LOCK_BITS: u32 = 0xC0;

pub fn locked_control_mask(mask: u32, locked: bool) -> u32 {
    if locked {
        mask | DOWNED_CONTROL_LOCK_BITS
    } else {
        mask
    }
}
