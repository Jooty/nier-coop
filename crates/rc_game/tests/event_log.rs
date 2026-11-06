use rc_game::event_log::{
    EVENT_LOG_DUPLICATE_WINDOW_MS, EVENT_LOG_MAX_ENTRIES, EVENT_LOG_MAX_TEXT_CHARS,
    EVENT_LOG_VISIBLE_MS, EventLog, event_log_layout,
};

#[test]
fn the_newest_message_comes_last() {
    let mut log = EventLog::new();
    log.post("first", 0);
    log.post("second", 100);
    let texts: Vec<_> = log
        .visible(1000)
        .into_iter()
        .map(|event| event.text)
        .collect();
    assert_eq!(texts, ["first", "second"]);
}

#[test]
fn a_message_fades_out_and_then_disappears() {
    let mut log = EventLog::new();
    log.post("gone soon", 0);
    assert_eq!(log.visible(1000)[0].alpha, 1.0);
    let fading = log.visible(EVENT_LOG_VISIBLE_MS - 600)[0].alpha;
    assert!(fading > 0.0 && fading < 1.0);
    assert!(log.visible(EVENT_LOG_VISIBLE_MS).is_empty());
}

#[test]
fn a_new_message_fades_in() {
    let mut log = EventLog::new();
    log.post("hello", 0);
    let start = log.visible(0)[0].alpha;
    assert!(start < 0.1);
}

#[test]
fn the_same_message_twice_in_a_row_shows_once() {
    let mut log = EventLog::new();
    assert!(log.post("Kaine was downed", 0));
    assert!(!log.post("Kaine was downed", EVENT_LOG_DUPLICATE_WINDOW_MS - 1));
    assert!(log.post("Kaine was downed", EVENT_LOG_DUPLICATE_WINDOW_MS));
}

#[test]
fn the_oldest_message_is_dropped_when_the_stack_is_full() {
    let mut log = EventLog::new();
    for number in 0..=EVENT_LOG_MAX_ENTRIES {
        log.post(&format!("message {number}"), number as u64);
    }
    let events = log.visible(100);
    assert_eq!(events.len(), EVENT_LOG_MAX_ENTRIES);
    assert_eq!(events[0].text, "message 1");
}

#[test]
fn control_characters_are_dropped_and_long_text_is_cut() {
    let mut log = EventLog::new();
    assert!(!log.post("\n\t", 0));
    log.post(&format!("a\nb{}", "x".repeat(200)), 0);
    let text = log.visible(500).remove(0).text;
    assert!(text.starts_with("ab"));
    assert_eq!(text.chars().count(), EVENT_LOG_MAX_TEXT_CHARS);
}

#[test]
fn the_stack_sits_at_the_bottom_left_and_grows_upwards() {
    let layout = event_log_layout(1080.0);
    assert!(layout.left > 0.0 && layout.left < 200.0);
    assert!(layout.bottom < 1080.0 && layout.bottom > 540.0);
    assert!(layout.line_top(1) < layout.line_top(0));
    assert_eq!(layout.line_top(0) + layout.line_height, layout.bottom);
}
