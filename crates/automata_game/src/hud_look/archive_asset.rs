/// Where one file sits inside a game archive, as the archive's own index lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveAsset {
    pub archive_offset: u64,
    pub stored_size: usize,
    pub unpacked_size: usize,
}
