#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasRect {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl AtlasRect {
    pub const fn new(left: u32, top: u32, right: u32, bottom: u32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn width(&self) -> u32 {
        self.right - self.left
    }

    pub const fn height(&self) -> u32 {
        self.bottom - self.top
    }
}
