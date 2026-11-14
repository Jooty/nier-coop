use crate::character::Character;

/// The menu's line about the body that stands for the other player here.
pub fn partner_body_status_line(wanted: Character, held_object_id: Option<u32>) -> String {
    let wanted_name = wanted.launcher_id().to_ascii_uppercase();
    match held_object_id {
        None => format!("partner body: none yet (the other player plays {wanted_name})"),
        Some(object_id) if object_id == wanted.object_id() => {
            format!("partner body: {wanted_name}")
        }
        Some(object_id) => {
            let shown = Character::from_model_id(object_id).map_or_else(
                || format!("object {object_id:#X}"),
                |character| character.launcher_id().to_ascii_uppercase(),
            );
            format!(
                "partner body: the engine's {shown} stands in for {wanted_name} (the data of {wanted_name} is not loaded in this place)"
            )
        }
    }
}
