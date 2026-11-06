#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArcAsset {
    pub archive_offset: u64,
    pub compressed_size: usize,
    pub decoded_size: usize,
}
