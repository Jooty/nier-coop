use std::collections::HashMap;

use nt_wire::{old_env_name, read_env_with_old_name};

fn getter(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect();
    move |key| map.get(key).cloned()
}

#[test]
fn the_old_name_swaps_the_prefix() {
    assert_eq!(
        old_env_name("NIERTOGETHER_BOOT_SLOT").as_deref(),
        Some("NIER_COOP_BOOT_SLOT")
    );
    assert_eq!(old_env_name("WINEPREFIX"), None);
}

#[test]
fn the_new_name_wins_over_the_old_one() {
    let read = read_env_with_old_name(
        "NIERTOGETHER_LINK",
        getter(&[("NIERTOGETHER_LINK", "1"), ("NIER_COOP_LINK", "2")]),
    );
    assert_eq!(read.value.as_deref(), Some("1"));
    assert_eq!(read.old_name_used, None);
}

#[test]
fn the_old_name_is_read_and_named_when_the_new_one_is_unset() {
    let read = read_env_with_old_name("NIERTOGETHER_LINK", getter(&[("NIER_COOP_LINK", "2")]));
    assert_eq!(read.value.as_deref(), Some("2"));
    assert_eq!(read.old_name_used.as_deref(), Some("NIER_COOP_LINK"));
}

#[test]
fn nothing_set_reads_nothing() {
    let read = read_env_with_old_name("NIERTOGETHER_LINK", getter(&[]));
    assert_eq!(read.value, None);
    assert_eq!(read.old_name_used, None);
}

#[test]
fn a_name_without_the_prefix_has_no_old_name() {
    let read = read_env_with_old_name("OTHER_LINK", getter(&[("NIER_COOP_LINK", "2")]));
    assert_eq!(read.value, None);
}
