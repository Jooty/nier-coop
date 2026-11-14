pub const fn new_name_or_old(
    new_name_value: Option<&'static str>,
    old_name_value: Option<&'static str>,
) -> Option<&'static str> {
    match new_name_value {
        Some(_) => new_name_value,
        None => old_name_value,
    }
}
