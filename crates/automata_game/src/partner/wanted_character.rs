use crate::character::Character;

pub fn wanted_partner(link_setting: Character, told_by_partner: Option<Character>) -> Character {
    told_by_partner.unwrap_or(link_setting)
}

pub fn partner_character_note(
    link_setting: Character,
    told_by_partner: Character,
) -> Option<String> {
    (link_setting != told_by_partner).then(|| {
        format!(
            "partner: the other player says it plays {}, the link setting said {}; making {}",
            told_by_partner.launcher_id(),
            link_setting.launcher_id(),
            told_by_partner.launcher_id()
        )
    })
}
