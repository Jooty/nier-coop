const EVENT_CALLBACK_PREFIXES: [&str; 4] = ["EID_", "SEID_", "RA_EID_", "snow_EID_"];
const EVENT_CALLBACK_SUFFIXES: [&str; 2] = ["_Start", "_End"];

pub fn is_event_callback_name(name: &str) -> bool {
    let without_digits = name.trim_end_matches(|character: char| character.is_ascii_digit());
    EVENT_CALLBACK_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
        && EVENT_CALLBACK_SUFFIXES
            .iter()
            .any(|suffix| without_digits.ends_with(suffix))
}

pub fn is_event_id_name(name: &str) -> bool {
    EVENT_CALLBACK_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}
