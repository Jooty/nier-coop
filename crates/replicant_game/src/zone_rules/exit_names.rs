use crate::zone_rules::ZONE_EXIT_CALLBACK_NAME_PARTS;

pub fn is_zone_exit_callback(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    ZONE_EXIT_CALLBACK_NAME_PARTS
        .iter()
        .any(|part| lowered.contains(part))
}
