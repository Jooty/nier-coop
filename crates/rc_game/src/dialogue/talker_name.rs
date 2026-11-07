use crate::dialogue::PLAYER_NAME_TAG;
use crate::player_name::fit_name_field;

pub fn replace_player_name_tag(text: &[u8], partner: &str) -> Option<Vec<u8>> {
    let name = fit_name_field(partner)?;
    let tag = PLAYER_NAME_TAG.as_bytes();
    let first = text.windows(tag.len()).position(|window| window == tag)?;
    let mut replaced = Vec::with_capacity(text.len() + name.len());
    replaced.extend_from_slice(&text[..first]);
    let mut rest = &text[first..];
    while let Some(position) = rest.windows(tag.len()).position(|window| window == tag) {
        replaced.extend_from_slice(&rest[..position]);
        replaced.extend_from_slice(name.as_bytes());
        rest = &rest[position + tag.len()..];
    }
    replaced.extend_from_slice(rest);
    Some(replaced)
}

pub fn speaker_name_to_show(shown: &str, own_name: &str, partner: Option<&str>) -> Option<String> {
    let shown = shown.trim();
    let is_protagonist =
        shown == PLAYER_NAME_TAG || (!own_name.trim().is_empty() && shown == own_name.trim());
    if !is_protagonist {
        return None;
    }
    fit_name_field(partner?)
}
