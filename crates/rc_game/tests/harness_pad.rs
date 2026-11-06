use rc_game::harness::{ScreenGuess, guess_screen, pad_button_bit};

#[test]
fn the_a_button_is_the_xinput_a_bit() {
    assert_eq!(pad_button_bit("A"), Some(0x1000));
}

#[test]
fn an_unknown_button_is_none() {
    assert_eq!(pad_button_bit("turbo"), None);
}

#[test]
fn a_player_with_an_active_map_is_in_game() {
    assert_eq!(guess_screen(true, true), ScreenGuess::InGame);
}

#[test]
fn no_player_and_no_map_is_title_or_menu() {
    assert_eq!(guess_screen(false, false), ScreenGuess::TitleOrMenu);
}
