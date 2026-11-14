use crate::loading_text::{MAX_PARTNER_NAME_CHARS, MAX_TEXT_BYTES, PARTNER_PLACEHOLDER};

pub fn clean_partner_name(name: &str) -> Option<String> {
    let cleaned: String = name
        .chars()
        .filter(|character| !character.is_control() && u32::from(*character) <= 0xFFFF)
        .take(MAX_PARTNER_NAME_CHARS)
        .collect();
    let trimmed = cleaned.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

pub fn fill_partner(template: &str, cleaned_partner: Option<&str>) -> Option<String> {
    let filled = if template.contains(PARTNER_PLACEHOLDER) {
        template.replace(PARTNER_PLACEHOLDER, cleaned_partner?)
    } else {
        template.to_owned()
    };
    let fits_the_screen = filled.len() <= MAX_TEXT_BYTES
        && filled
            .chars()
            .all(|character| !character.is_control() && u32::from(character) <= 0xFFFF);
    fits_the_screen.then_some(filled)
}
