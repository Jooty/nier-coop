use crate::story_progress::{
    RANK_SIDECAR_BYTES, RANK_SIDECAR_MAGIC, RANK_SIDECAR_VERSION, StoryRank,
};

/// The `GAMEDATA.coop.rank` sidecar: magic, version, part letter (0 = none), two zero bytes,
/// milestone count (u32 little endian), four zero bytes.
pub fn encode_rank_sidecar(rank: StoryRank) -> [u8; RANK_SIDECAR_BYTES] {
    let mut bytes = [0_u8; RANK_SIDECAR_BYTES];
    bytes[..4].copy_from_slice(&RANK_SIDECAR_MAGIC);
    bytes[4] = RANK_SIDECAR_VERSION;
    bytes[5] = rank.part.unwrap_or(0);
    bytes[8..12].copy_from_slice(&rank.milestones.to_le_bytes());
    bytes
}

pub fn decode_rank_sidecar(bytes: &[u8]) -> Option<StoryRank> {
    if bytes.len() != RANK_SIDECAR_BYTES
        || bytes[..4] != RANK_SIDECAR_MAGIC
        || bytes[4] != RANK_SIDECAR_VERSION
    {
        return None;
    }
    let milestones = u32::from_le_bytes(bytes[8..12].try_into().ok()?);
    Some(StoryRank {
        part: (bytes[5] != 0).then_some(bytes[5]),
        milestones,
    })
}
