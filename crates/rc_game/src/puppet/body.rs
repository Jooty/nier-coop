use rc_proto::PartnerBody;

pub const BODY_TABLE: [BodyInfo; 2] = [
    BodyInfo {
        body: PartnerBody::Nier,
        pack_name: "NIER013",
        idle_motion: "NIER013_idle_lp",
        weapon_shown: true,
        request_kind: 0,
        request_slot_hint: 1,
    },
    BodyInfo {
        body: PartnerBody::PrologueCharacter,
        pack_name: "NIER014",
        idle_motion: "NIER013_idle_lp",
        weapon_shown: true,
        request_kind: 0,
        request_slot_hint: 1,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyInfo {
    pub body: PartnerBody,
    pub pack_name: &'static str,
    pub idle_motion: &'static str,
    pub weapon_shown: bool,
    pub request_kind: i32,
    pub request_slot_hint: i32,
}

pub fn body_info(body: PartnerBody) -> &'static BodyInfo {
    BODY_TABLE
        .iter()
        .find(|info| info.body == body)
        .unwrap_or(&BODY_TABLE[0])
}
