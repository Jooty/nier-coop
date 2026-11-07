use rc_proto::{GroupRoster, MAX_DESYNC_GROUPS};

const FNV_OFFSET_BASIS: u32 = 0x811c_9dc5;
const FNV_PRIME: u32 = 0x0100_0193;

pub fn ids_hash(ids: &[i32]) -> u32 {
    let mut sorted = ids.to_vec();
    sorted.sort_unstable();
    sorted.iter().fold(FNV_OFFSET_BASIS, |hash, id| {
        id.to_le_bytes().iter().fold(hash, |hash, byte| {
            (hash ^ u32::from(*byte)).wrapping_mul(FNV_PRIME)
        })
    })
}

pub fn live_group_rosters(live: &[(u16, i32)]) -> (Vec<GroupRoster>, u16) {
    let mut groups: Vec<u16> = live.iter().map(|(group, _)| *group).collect();
    groups.sort_unstable();
    groups.dedup();
    let mut rosters: Vec<GroupRoster> = groups
        .iter()
        .map(|&group| {
            let ids: Vec<i32> = live
                .iter()
                .filter(|(member_group, _)| *member_group == group)
                .map(|(_, id)| *id)
                .collect();
            GroupRoster {
                group,
                count: u16::try_from(ids.len()).unwrap_or(u16::MAX),
                ids_hash: ids_hash(&ids),
            }
        })
        .collect();
    let not_listed = rosters.len().saturating_sub(MAX_DESYNC_GROUPS);
    rosters.truncate(MAX_DESYNC_GROUPS);
    (rosters, u16::try_from(not_listed).unwrap_or(u16::MAX))
}
