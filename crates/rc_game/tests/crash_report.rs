use rc_game::crash_report::{
    ACCESS_VIOLATION_CODE, BREAKPOINT_CODE, CrashFacts, ExitCaller, ModuleSpot, StackFrame,
    access_violation_kind, at_exit_line, detach_line, early_line, exit_call_line, is_fatal_code,
    is_return_address_candidate, last_lines, panic_line, report_text,
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
        stack: vec![StackFrame {
            address: 0x1_4000_2000,
            module: ModuleSpot {
                name: "game.exe".to_owned(),
                base: 0x1_4000_0000,
            },
        }],
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

#[test]
fn the_stack_frames_are_listed_with_module_and_rva() {
    let text = report_text(&facts(false, false), &[]);
    assert!(text.contains("crash: stack 0 0x0000000140002000 game.exe rva 0x2000\n"));
}

#[test]
fn breakpoints_are_fatal_and_the_early_line_names_code_and_thread() {
    assert!(is_fatal_code(BREAKPOINT_CODE));
    assert_eq!(
        early_line(0xC000_00FD, 0x1_4000_0010, 9),
        "crash: fatal exception code 0xC00000FD at 0x0000000140000010 on thread 9"
    );
}

#[test]
fn only_words_inside_a_module_span_are_return_address_candidates() {
    let bases = [0x1_4000_0000, 0x1_8000_0000];
    assert!(is_return_address_candidate(
        0x1_4000_0001,
        &bases,
        0x100_0000
    ));
    assert!(is_return_address_candidate(
        0x1_80ff_ffff,
        &bases,
        0x100_0000
    ));
    assert!(!is_return_address_candidate(
        0x1_4100_0000,
        &bases,
        0x100_0000
    ));
    assert!(!is_return_address_candidate(0x10, &bases, 0x100_0000));
}

#[test]
fn a_panic_line_names_the_message_the_place_and_the_thread() {
    assert_eq!(
        panic_line("index out of bounds", Some(("src/a.rs", 12, 5)), "main"),
        "panic: index out of bounds at src/a.rs:12:5 (thread main)"
    );
    assert_eq!(
        panic_line("boom", None, "ThreadId(4)"),
        "panic: boom at an unknown place (thread ThreadId(4))"
    );
}

#[test]
fn the_detach_line_tells_a_process_exit_from_an_unload() {
    assert!(detach_line(true, 3).contains("unloaded with FreeLibrary"));
    assert!(detach_line(false, 3).contains("game process is ending"));
    assert!(detach_line(false, 3).ends_with("on thread 3"));
}

#[test]
fn an_exit_call_line_names_the_api_code_thread_and_caller() {
    let caller = ExitCaller {
        module: "game.exe",
        rva: 0x1A2B,
    };
    let line = exit_call_line("TerminateProcess", 1, 8, Some(caller));
    assert!(line.contains("TerminateProcess called with exit code 1 on thread 8"));
    assert!(line.ends_with("in game.exe rva 0x1A2B"));
    assert!(exit_call_line("ExitProcess", 0, 8, None).ends_with("in no known module"));
}

#[test]
fn the_at_exit_line_names_the_thread() {
    assert!(at_exit_line(5).contains("exit handlers are running on thread 5"));
}
