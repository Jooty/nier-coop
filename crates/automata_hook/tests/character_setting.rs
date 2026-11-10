use automata_game::character::Character;
use automata_hook::character_setting::choose_character;

fn text(value: &str) -> Option<String> {
    Some(value.to_owned())
}

#[test]
fn the_launcher_character_is_used_as_given() {
    let choice = choose_character(None, None, Some("role = \"guest\"\ncharacter = \"a2\"\n"));
    assert_eq!(choice.character, Character::A2);
    assert_eq!(choice.log_line, "character: a2 from the launcher (guest)");
}

#[test]
fn the_environment_wins_over_the_link_file() {
    let choice = choose_character(
        text("9s"),
        text("guest"),
        Some("role = \"host\"\ncharacter = \"2b\"\n"),
    );
    assert_eq!(choice.character, Character::NineS);
}

#[test]
fn a_missing_value_falls_back_to_2b_for_the_host_and_9s_for_the_guest() {
    let host = choose_character(None, text("host"), None);
    assert_eq!(host.character, Character::TwoB);
    assert!(host.log_line.contains("gave none"));
    let guest = choose_character(None, text("guest"), Some("link = 4000\n"));
    assert_eq!(guest.character, Character::NineS);
    assert!(guest.log_line.contains("gave none"));
}

#[test]
fn an_unknown_value_logs_a_plain_line_and_falls_back() {
    let host = choose_character(text("kaine"), text("host"), None);
    assert_eq!(host.character, Character::TwoB);
    assert!(host.log_line.contains("unknown value \"kaine\""));
    let guest = choose_character(text("kaine"), text("guest"), None);
    assert_eq!(guest.character, Character::NineS);
}

#[test]
fn a_file_that_is_not_toml_counts_as_missing() {
    let choice = choose_character(None, None, Some("not toml ["));
    assert_eq!(choice.character, Character::TwoB);
}
