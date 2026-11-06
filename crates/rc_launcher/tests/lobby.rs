use rc_launcher::lobby::{
    LaunchMode, MAX_NAME_BYTES, NameError, ReadyInputs, lobby_ready, new_game_slot, resolve_mode,
    start_enabled, validate_name,
};
use rc_launcher::net::{LOBBY_MODE_CONTINUE, LOBBY_MODE_NEW_GAME, LobbyReady};
use rc_launcher::saves::SlotSummary;

fn slot(index: u8, used: bool) -> SlotSummary {
    SlotSummary {
        index,
        used,
        name: String::new(),
        phase: String::new(),
        level: 1,
        play_seconds: 0.0,
    }
}

fn slots_with_used(used: &[u8]) -> Vec<SlotSummary> {
    (0..7)
        .map(|index| slot(index, used.contains(&index)))
        .collect()
}

fn ready(ready: bool, mode: u8) -> LobbyReady {
    LobbyReady {
        ready,
        mode,
        slot: None,
    }
}

fn inputs(mode: u8) -> ReadyInputs {
    ReadyInputs {
        mode,
        game_ok: true,
        name_ok: true,
        new_game_slot_free: true,
        continue_slot: None,
        is_guest: false,
    }
}

#[test]
fn a_name_is_trimmed_and_kept() {
    assert_eq!(validate_name("  Kit_2-b  ").unwrap(), "Kit_2-b");
}

#[test]
fn an_empty_name_is_refused() {
    assert_eq!(validate_name("   "), Err(NameError::Empty));
}

#[test]
fn a_name_over_the_limit_is_refused() {
    let longest = "a".repeat(MAX_NAME_BYTES);
    assert!(validate_name(&longest).is_ok());
    assert_eq!(
        validate_name(&format!("{longest}a")),
        Err(NameError::TooLong)
    );
}

#[test]
fn a_name_with_other_characters_is_refused() {
    assert_eq!(validate_name("Nier!"), Err(NameError::BadCharacter));
    assert_eq!(validate_name("Niér"), Err(NameError::BadCharacter));
}

#[test]
fn a_new_game_takes_the_first_empty_slot() {
    assert_eq!(new_game_slot(&slots_with_used(&[0, 1])), Some(2));
}

#[test]
fn a_new_game_with_no_save_file_takes_slot_zero() {
    assert_eq!(new_game_slot(&[]), Some(0));
}

#[test]
fn a_new_game_never_takes_the_seventh_slot() {
    assert_eq!(new_game_slot(&slots_with_used(&[0, 1, 2, 3, 4, 5])), None);
}

#[test]
fn only_the_host_can_start_and_only_when_both_are_ready() {
    let yes = ready(true, LOBBY_MODE_NEW_GAME);
    let no = ready(false, LOBBY_MODE_NEW_GAME);
    assert!(start_enabled(true, false, false, yes, Some(yes)));
    assert!(!start_enabled(true, false, false, yes, Some(no)));
    assert!(!start_enabled(true, false, false, no, Some(yes)));
    assert!(!start_enabled(true, false, false, yes, None));
    assert!(!start_enabled(true, true, false, yes, Some(yes)));
    assert!(!start_enabled(false, false, false, yes, Some(yes)));
}

#[test]
fn start_is_disabled_once_the_game_was_started() {
    let yes = ready(true, LOBBY_MODE_NEW_GAME);
    assert!(!start_enabled(true, false, true, yes, Some(yes)));
}

#[test]
fn a_new_game_needs_a_name_a_free_slot_and_the_game() {
    assert!(lobby_ready(inputs(LOBBY_MODE_NEW_GAME)).ready);
    let no_name = ReadyInputs {
        name_ok: false,
        ..inputs(LOBBY_MODE_NEW_GAME)
    };
    assert!(!lobby_ready(no_name).ready);
    let no_slot = ReadyInputs {
        new_game_slot_free: false,
        ..inputs(LOBBY_MODE_NEW_GAME)
    };
    assert!(!lobby_ready(no_slot).ready);
    let no_game = ReadyInputs {
        game_ok: false,
        ..inputs(LOBBY_MODE_NEW_GAME)
    };
    assert!(!lobby_ready(no_game).ready);
}

#[test]
fn the_host_continuing_needs_a_picked_slot() {
    let none = ReadyInputs {
        name_ok: false,
        ..inputs(LOBBY_MODE_CONTINUE)
    };
    assert!(!lobby_ready(none).ready);
    let picked = ReadyInputs {
        continue_slot: Some(3),
        ..none
    };
    let state = lobby_ready(picked);
    assert!(state.ready);
    assert_eq!(state.slot, Some(3));
}

#[test]
fn a_guest_without_a_save_can_still_join_a_continue_as_a_new_game() {
    let guest = ReadyInputs {
        is_guest: true,
        ..inputs(LOBBY_MODE_CONTINUE)
    };
    assert!(lobby_ready(guest).ready);
    let host = ReadyInputs {
        is_guest: false,
        ..guest
    };
    assert!(!lobby_ready(host).ready);
}

#[test]
fn continuing_uses_the_picked_slot() {
    assert_eq!(
        resolve_mode(LOBBY_MODE_CONTINUE, Some(2), Some("Kit".to_owned())),
        Some(LaunchMode::Continue { slot: 2 })
    );
}

#[test]
fn continuing_without_a_slot_starts_a_new_game_with_the_name() {
    assert_eq!(
        resolve_mode(LOBBY_MODE_CONTINUE, None, Some("Kit".to_owned())),
        Some(LaunchMode::NewGame {
            name: "Kit".to_owned()
        })
    );
    assert_eq!(resolve_mode(LOBBY_MODE_CONTINUE, None, None), None);
}

#[test]
fn a_new_game_ignores_the_picked_slot() {
    assert_eq!(
        resolve_mode(LOBBY_MODE_NEW_GAME, Some(2), Some("Kit".to_owned())),
        Some(LaunchMode::NewGame {
            name: "Kit".to_owned()
        })
    );
}
