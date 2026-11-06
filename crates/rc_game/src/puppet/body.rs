use rc_proto::PartnerBody;

pub const BODY_TABLE: [BodyInfo; 2] = [
    BodyInfo {
        body: PartnerBody::Nier,
        pack_name: "NIER013",
        idle_motions: &["NIER013_idle_lp"],
        weapon_shown: true,
        request_kind: 0,
        request_slot_hint: 1,
        own_seed_name: "nierB",
        own_seed_table_index: 0,
        own_body_changes: "",
    },
    BodyInfo {
        body: PartnerBody::PrologueCharacter,
        pack_name: "NIER014",
        idle_motions: &["NIER013_idle_lp", "NIER014_idle_lp"],
        weapon_shown: true,
        request_kind: 0,
        request_slot_hint: 1,
        own_seed_name: "nierT",
        own_seed_table_index: 1,
        own_body_changes: "book companion bookw011 becomes bookb012, extra pack nier019 is not loaded, spec class PlayerBoy becomes PlayerDefault",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyInfo {
    pub body: PartnerBody,
    pub pack_name: &'static str,
    pub idle_motions: &'static [&'static str],
    pub weapon_shown: bool,
    pub request_kind: i32,
    pub request_slot_hint: i32,
    pub own_seed_name: &'static str,
    pub own_seed_table_index: u32,
    pub own_body_changes: &'static str,
}

pub fn body_info(body: PartnerBody) -> &'static BodyInfo {
    BODY_TABLE
        .iter()
        .find(|info| info.body == body)
        .unwrap_or(&BODY_TABLE[0])
}
