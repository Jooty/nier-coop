use rc_proto::PartnerBody;

const NIER_LABEL: &str = "Nier (follows the story)";
const PROLOGUE_CHARACTER_LABEL: &str = "The prologue character";
const KAINE_LABEL: &str = "Kainé";

pub fn body_label(body: PartnerBody) -> &'static str {
    match body {
        PartnerBody::PrologueCharacter => PROLOGUE_CHARACTER_LABEL,
        PartnerBody::SeedKaineE => KAINE_LABEL,
        PartnerBody::Nier | PartnerBody::SeedNierY | PartnerBody::SeedNierF => NIER_LABEL,
    }
}
