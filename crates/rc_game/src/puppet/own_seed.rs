use rc_proto::PartnerBody;

use crate::puppet::body_info;

pub const SEED_NAME_TABLE_STRIDE: u64 = 0x10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnSeedSwap {
    pub from: &'static str,
    pub to: &'static str,
    pub table_index: u32,
    pub changes: &'static str,
}

pub fn own_seed_swap(returned_seed: &str, body: PartnerBody) -> Option<OwnSeedSwap> {
    let nier = body_info(PartnerBody::Nier);
    let wanted = body_info(body);
    (body != PartnerBody::Nier && returned_seed == nier.own_seed_name).then_some(OwnSeedSwap {
        from: nier.own_seed_name,
        to: wanted.own_seed_name,
        table_index: wanted.own_seed_table_index,
        changes: wanted.own_body_changes,
    })
}
