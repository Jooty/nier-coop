use nt_hook::loading_text::{
    LoadingText, MAX_TEXT_BYTES, SHOW_ONE_LOADING_SCREEN_IN, SessionPartner, clean_partner_name,
    fill_partner,
};

const LINES: [&str; 3] = [
    "{partner} walked beside me.",
    "Two sets of footprints.",
    "I told {partner} to stay close.",
];

fn lines() -> LoadingText {
    LoadingText::new(&LINES)
}

fn every_screen_roll(pick: u64) -> u64 {
    pick * SHOW_ONE_LOADING_SCREEN_IN
}

#[test]
fn nothing_is_shown_outside_a_co_op_session() {
    let mut text = lines();
    for roll in 0..30 {
        assert_eq!(text.choose_with_roll(SessionPartner::NoSession, roll), None);
    }
    assert_eq!(
        SessionPartner::from_session(false, Some("Ayla")),
        SessionPartner::NoSession
    );
}

#[test]
fn a_session_with_a_name_fills_the_partner_in() {
    let mut text = lines();
    let choice = text
        .choose_with_roll(SessionPartner::Named("Ayla"), every_screen_roll(0))
        .expect("shown");
    assert_eq!(choice.text, "Ayla walked beside me.");
    assert_eq!(choice.line_index, 0);
    assert_eq!(choice.partner_name_chars, 4);
}

#[test]
fn lines_that_name_the_partner_are_skipped_without_one() {
    let mut text = lines();
    for pick in 0..12 {
        let choice = text
            .choose_with_roll(SessionPartner::NoPartnerYet, every_screen_roll(pick))
            .expect("the plain line is still shown");
        assert_eq!(choice.text, "Two sets of footprints.");
        assert_eq!(choice.partner_name_chars, 0);
    }
    assert_eq!(
        SessionPartner::from_session(true, Some("  ")),
        SessionPartner::NoPartnerYet
    );
    assert_eq!(
        SessionPartner::from_session(true, None),
        SessionPartner::NoPartnerYet
    );
}

#[test]
fn only_about_one_screen_in_three_shows_a_line() {
    let mut text = lines();
    let shown = (0..3000)
        .filter(|_| text.choose(SessionPartner::Named("Ayla")).is_some())
        .count();
    assert!((800..1200).contains(&shown), "shown {shown} of 3000");
}

#[test]
fn a_roll_that_misses_leaves_the_game_its_own_text() {
    let mut text = lines();
    for roll in [1, 2, 4, 5, 7, 8] {
        assert_eq!(
            text.choose_with_roll(SessionPartner::Named("Ayla"), roll),
            None
        );
    }
}

#[test]
fn the_same_line_never_comes_twice_in_a_row() {
    let mut text = lines();
    let mut previous = None;
    for pick in 0..200 {
        let choice = text
            .choose_with_roll(SessionPartner::Named("Ayla"), every_screen_roll(pick / 2))
            .expect("shown");
        assert_ne!(Some(choice.line_index), previous);
        previous = Some(choice.line_index);
    }
}

#[test]
fn the_rotation_reaches_every_line() {
    let mut text = lines();
    let mut seen = [false; 3];
    for _ in 0..300 {
        if let Some(choice) = text.choose(SessionPartner::Named("Ayla")) {
            seen[choice.line_index] = true;
        }
    }
    assert_eq!(seen, [true; 3]);
}

#[test]
fn a_switched_off_setting_shows_nothing() {
    let mut text = lines();
    text.set_enabled(false);
    for pick in 0..30 {
        assert_eq!(
            text.choose_with_roll(SessionPartner::Named("Ayla"), every_screen_roll(pick)),
            None
        );
    }
    text.set_enabled(true);
    assert!(
        text.choose_with_roll(SessionPartner::Named("Ayla"), every_screen_roll(0))
            .is_some()
    );
}

#[test]
fn a_partner_name_is_cleaned_to_what_the_screen_can_draw() {
    assert_eq!(clean_partner_name("  Ayla \n"), Some("Ayla".to_owned()));
    assert_eq!(clean_partner_name("A\u{1F600}b"), Some("Ab".to_owned()));
    assert_eq!(clean_partner_name("\u{1F600}"), None);
    let long = "n".repeat(100);
    assert_eq!(
        clean_partner_name(&long).map(|name| name.chars().count()),
        Some(31)
    );
}

#[test]
fn text_the_screen_cannot_hold_is_refused() {
    assert_eq!(fill_partner("Plain.", None), Some("Plain.".to_owned()));
    assert_eq!(
        fill_partner("{partner} and {partner}.", Some("Ayla")),
        Some("Ayla and Ayla.".to_owned())
    );
    assert_eq!(fill_partner("{partner}.", None), None);
    assert_eq!(fill_partner("two\nlines", None), None);
    assert_eq!(
        fill_partner("outside the basic plane \u{1F600}", None),
        None
    );
    assert_eq!(fill_partner(&"a".repeat(MAX_TEXT_BYTES + 1), None), None);
    assert!(fill_partner(&"a".repeat(MAX_TEXT_BYTES), None).is_some());
}
