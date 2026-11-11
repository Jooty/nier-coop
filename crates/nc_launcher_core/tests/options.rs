use nc_launcher_core::{SessionOptions, options_differ_from_last};

fn hidden_names() -> SessionOptions {
    SessionOptions {
        show_names: false,
        ..SessionOptions::default()
    }
}

#[test]
fn the_first_options_always_count_as_changed() {
    let mut last = None;
    assert!(options_differ_from_last(
        &mut last,
        SessionOptions::default()
    ));
}

#[test]
fn the_same_options_sent_again_are_not_a_change() {
    let mut last = None;
    assert!(options_differ_from_last(&mut last, hidden_names()));
    assert!(!options_differ_from_last(&mut last, hidden_names()));
    assert!(!options_differ_from_last(&mut last, hidden_names()));
}

#[test]
fn different_options_are_a_change_again() {
    let mut last = None;
    options_differ_from_last(&mut last, hidden_names());
    assert!(options_differ_from_last(
        &mut last,
        SessionOptions::default()
    ));
    assert_eq!(last, Some(SessionOptions::default()));
}
