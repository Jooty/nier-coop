use crate::character::{Character, pick_distinct};

/// The character of the partner body: what the other player named, unless that is our own
/// body's character; never the character of our own body.
pub fn wanted_partner(
    link_setting: Character,
    told_by_partner: Option<Character>,
    local_body: Option<Character>,
) -> Character {
    let wanted = told_by_partner
        .filter(|told| Some(*told) != local_body)
        .unwrap_or(link_setting);
    match local_body {
        Some(local) => pick_distinct(wanted, local),
        None => wanted,
    }
}

pub fn partner_character_note(
    link_setting: Character,
    told_by_partner: Character,
) -> Option<String> {
    (link_setting != told_by_partner).then(|| {
        format!(
            "partner: the other player says it plays {}, the link setting said {}; making {} (unless that is our own body's character)",
            told_by_partner.launcher_id(),
            link_setting.launcher_id(),
            told_by_partner.launcher_id()
        )
    })
}
