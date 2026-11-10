pub const WM_CLOSE: u32 = 0x0010;
pub const WM_DESTROY: u32 = 0x0002;

pub fn starts_closing(message: u32) -> bool {
    message == WM_CLOSE || message == WM_DESTROY
}

pub fn actors_to_destroy(closing: bool, guarded: Vec<u64>) -> Vec<u64> {
    if closing {
        return Vec::new();
    }
    guarded.into_iter().filter(|actor| *actor != 0).collect()
}
