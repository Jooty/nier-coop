use automata_game::second_copy::{
    ERROR_ALREADY_EXISTS, EventTreatment, GAME_EVENT_NAME, event_treatment, must_rename,
    unique_event_name,
};

#[test]
fn only_the_games_own_event_name_is_renamed() {
    assert_eq!(
        event_treatment(Some(b"NieR:AutomataEvent")),
        EventTreatment::RenameWhenAlreadyExists
    );
    assert_eq!(
        event_treatment(Some(b"SomeOtherEvent")),
        EventTreatment::PassThrough
    );
    assert_eq!(
        event_treatment(Some(b"NieR:AutomataEvent.7")),
        EventTreatment::PassThrough
    );
    assert_eq!(event_treatment(Some(b"")), EventTreatment::PassThrough);
}

#[test]
fn an_unnamed_event_passes_through() {
    assert_eq!(event_treatment(None), EventTreatment::PassThrough);
}

#[test]
fn the_rename_needs_the_name_and_the_already_exists_error() {
    let renamed = EventTreatment::RenameWhenAlreadyExists;
    assert!(must_rename(renamed, ERROR_ALREADY_EXISTS));
    assert!(!must_rename(renamed, 0));
    assert!(!must_rename(
        EventTreatment::PassThrough,
        ERROR_ALREADY_EXISTS
    ));
}

#[test]
fn the_already_exists_error_is_the_win32_value() {
    assert_eq!(ERROR_ALREADY_EXISTS, 183);
}

#[test]
fn the_per_process_name_differs_by_process_and_never_equals_the_games_name() {
    let first = unique_event_name(100);
    let second = unique_event_name(200);
    assert_ne!(first, second);
    assert_ne!(first, GAME_EVENT_NAME);
    assert_eq!(first, "NieR:AutomataEvent.100");
    assert_eq!(
        event_treatment(Some(first.as_bytes())),
        EventTreatment::PassThrough
    );
}
