use automata_game::exe::{
    COMPAT_EXE, ExeFacts, ExeVariant, KNOWN_EXES, NEW_MAIN_EXE, OLD_MAIN_EXE, identify,
};

fn facts_of(exe: &automata_game::exe::ExeIdentity) -> ExeFacts {
    ExeFacts {
        file_size: exe.file_size,
        time_date_stamp: exe.time_date_stamp,
        sha256_hex: exe.sha256_hex.to_owned(),
    }
}

#[test]
fn each_known_exe_gives_its_variant() {
    assert_eq!(
        identify(&facts_of(&NEW_MAIN_EXE)).map(|exe| exe.variant),
        Some(ExeVariant::Main)
    );
    assert_eq!(
        identify(&facts_of(&OLD_MAIN_EXE)).map(|exe| exe.variant),
        Some(ExeVariant::Main)
    );
    assert_eq!(
        identify(&facts_of(&COMPAT_EXE)).map(|exe| exe.variant),
        Some(ExeVariant::Compat)
    );
}

#[test]
fn the_known_table_matches_the_pinned_build() {
    assert_eq!(NEW_MAIN_EXE.file_size, 17_613_568);
    assert_eq!(OLD_MAIN_EXE.file_size, 17_601_536);
    assert_eq!(COMPAT_EXE.file_size, 17_550_336);
    assert_eq!(NEW_MAIN_EXE.time_date_stamp, 0x6AC8_9519);
    assert_eq!(COMPAT_EXE.time_date_stamp, 0x6AC8_9501);
    assert_eq!(KNOWN_EXES.len(), 3);
}

#[test]
fn a_changed_size_is_refused() {
    for exe in KNOWN_EXES {
        let mut facts = facts_of(&exe);
        facts.file_size += 1;
        assert_eq!(identify(&facts), None);
    }
}

#[test]
fn a_changed_timestamp_is_refused() {
    for exe in KNOWN_EXES {
        let mut facts = facts_of(&exe);
        facts.time_date_stamp += 1;
        assert_eq!(identify(&facts), None);
    }
}

#[test]
fn a_changed_hash_is_refused() {
    for exe in KNOWN_EXES {
        let mut facts = facts_of(&exe);
        facts.sha256_hex.replace_range(..1, "f");
        if facts.sha256_hex == exe.sha256_hex {
            facts.sha256_hex.replace_range(..1, "0");
        }
        assert_eq!(identify(&facts), None);
    }
}

#[test]
fn size_and_hash_of_one_exe_do_not_pass_with_another_exes_timestamp() {
    let mut facts = facts_of(&NEW_MAIN_EXE);
    facts.time_date_stamp = COMPAT_EXE.time_date_stamp;
    assert_eq!(identify(&facts), None);
}

#[test]
fn the_hash_compare_ignores_letter_case() {
    let mut facts = facts_of(&NEW_MAIN_EXE);
    facts.sha256_hex = facts.sha256_hex.to_uppercase();
    assert_eq!(identify(&facts), Some(&NEW_MAIN_EXE));
}
