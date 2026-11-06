use rc_game::harness::{KEY_CODES, lookup_key};

#[test]
fn w_maps_to_its_scan_code_and_virtual_key() {
    let key = lookup_key("W").expect("w is a known key");
    assert_eq!((key.direct_input_scan_code, key.virtual_key), (0x11, 0x57));
}

#[test]
fn esc_is_an_alias_of_escape() {
    assert_eq!(lookup_key("esc"), lookup_key("escape"));
}

#[test]
fn an_unknown_name_is_none() {
    assert_eq!(lookup_key("banana"), None);
}

#[test]
fn no_two_keys_share_a_name_or_a_scan_code() {
    for (index, key) in KEY_CODES.iter().enumerate() {
        for other in &KEY_CODES[index + 1..] {
            assert_ne!(key.name, other.name);
            assert_ne!(key.direct_input_scan_code, other.direct_input_scan_code);
        }
    }
}
