use nc_launcher_core::{GameProfile, REPLICANT, character_choices};
use replicant_proto::PartnerBody;

const KAINE_LABEL: &str = "Kainé";
const NIER_CHARACTER_ID: &str = "nier";
const PROLOGUE_CHARACTER_ID: &str = "prologue";

pub fn character_id_for_body(body: PartnerBody) -> Option<&'static str> {
    match body {
        PartnerBody::PrologueCharacter => Some(PROLOGUE_CHARACTER_ID),
        PartnerBody::Nier | PartnerBody::SeedNierY | PartnerBody::SeedNierF => {
            Some(NIER_CHARACTER_ID)
        }
        PartnerBody::SeedKaineE => None,
    }
}

pub fn body_for_character_id(id: &str) -> Option<PartnerBody> {
    match id {
        NIER_CHARACTER_ID => Some(PartnerBody::Nier),
        PROLOGUE_CHARACTER_ID => Some(PartnerBody::PrologueCharacter),
        _ => None,
    }
}

pub fn body_label(body: PartnerBody) -> &'static str {
    character_id_for_body(body)
        .and_then(|id| REPLICANT.character(id))
        .map_or(KAINE_LABEL, |character| character.label)
}

pub fn picker_choices(profile: &GameProfile, taken_by_others: &[PartnerBody]) -> Vec<PartnerBody> {
    let taken: Vec<&str> = taken_by_others
        .iter()
        .filter_map(|body| character_id_for_body(*body))
        .collect();
    character_choices(profile, &taken)
        .into_iter()
        .filter_map(|character| body_for_character_id(character.id))
        .collect()
}
