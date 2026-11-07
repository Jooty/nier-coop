use rc_game::crash_report::{
    ACCESS_VIOLATION_CODE, CrashFacts, ModuleSpot, access_violation_kind, is_fatal_code,
    last_lines, report_text,
};

fn facts(in_our_module: bool, last_chance: bool) -> CrashFacts {
    CrashFacts {
        code: ACCESS_VIOLATION_CODE,
        address: 0x1_8000_1234,
        thread_id: 77,
        module: Some(ModuleSpot {
            name: "rc_hook.dll".to_owned(),
            base: 0x1_8000_0000,
        }),
        in_our_module,
        access: Some((1, 0x10)),
        last_chance,
    }
}

#[test]
fn the_report_names_code_address_module_rva_thread_and_access() {
    let text = report_text(&facts(true, false), &["one", "two"]);
    assert!(text.contains("code 0xC0000005 at 0x0000000180001234 on thread 77"));
    assert!(text.contains("module rc_hook.dll base 0x0000000180000000 rva 0x1234 (our hook)"));
    assert!(text.contains("write of 0x0000000000000010"));
    assert!(text.contains("crash:   one\ncrash:   two\n"));
    assert!(text.contains("exception in our own module"));
}

#[test]
fn a_last_chance_report_in_another_module_says_so() {
    let mut other = facts(false, true);
    other.access = None;
    let text = report_text(&other, &[]);
    assert!(text.contains("unhandled exception"));
    assert!(!text.contains("(our hook)"));
    assert!(!text.contains("write of"));
}

#[test]
fn an_address_in_no_module_is_reported() {
    let mut nowhere = facts(false, true);
    nowhere.module = None;
    assert!(report_text(&nowhere, &[]).contains("no loaded module"));
}

#[test]
fn only_the_last_lines_are_kept() {
    let text = "a\nb\nc\nd\n";
    assert_eq!(last_lines(text, 2), vec!["c", "d"]);
    assert_eq!(last_lines(text, 10), vec!["a", "b", "c", "d"]);
    assert!(last_lines("", 3).is_empty());
}

#[test]
fn only_real_faults_are_fatal_and_access_kinds_have_names() {
    assert!(is_fatal_code(ACCESS_VIOLATION_CODE));
    assert!(!is_fatal_code(0x406D_1388));
    assert!(!is_fatal_code(0xE06D_7363));
    assert_eq!(access_violation_kind(0), "read");
    assert_eq!(access_violation_kind(8), "execute");
    assert_eq!(access_violation_kind(5), "other");
}

#[test]
fn a_first_chance_report_in_another_module_says_so() {
    let text = report_text(&facts(false, false), &[]);
    assert!(text.contains("first-chance fatal exception in another module"));
    assert!(!text.contains("(our hook)"));
}
