use crate::player_name::NAME_FIELD_LEN;

pub fn fit_name_field(name: &str) -> Option<String> {
    let mut fitted = String::new();
    for character in name.chars() {
        if character.is_control() || u32::from(character) > 0xFFFF {
            continue;
        }
        if fitted.len() + character.len_utf8() >= NAME_FIELD_LEN {
            break;
        }
        fitted.push(character);
    }
    let trimmed = fitted.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

pub fn name_field_bytes(name: &str) -> Option<[u8; NAME_FIELD_LEN]> {
    let fitted = fit_name_field(name)?;
    let mut field = [0_u8; NAME_FIELD_LEN];
    field[..fitted.len()].copy_from_slice(fitted.as_bytes());
    Some(field)
}

pub fn local_name(configured: Option<&str>, save_block: &str) -> Option<String> {
    configured
        .and_then(fit_name_field)
        .or_else(|| fit_name_field(save_block))
}

pub fn diary_name(partner: Option<&str>) -> Option<[u8; NAME_FIELD_LEN]> {
    name_field_bytes(partner?)
}
