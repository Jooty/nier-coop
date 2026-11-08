use rc_launcher::lobby::{
    GUEST_WAIT_FOR_HOST_GAME_SECONDS, GameState, GuestWaitStep, GuestWaiting, LaunchMode,
    MAX_NAME_BYTES, NameError, OwnGameInputs, PlayerRole, ReadyInputs, RowInputs,
    guest_accepts_start, host_options_editable, lobby_ready, new_game_slot, own_game_state,
    partner_game_state, picker_choices, player_rows, resolve_mode, start_enabled, validate_name,
};
use rc_launcher::net::{LOBBY_MODE_CONTINUE, LOBBY_MODE_NEW_GAME, LobbyReady};
use rc_launcher::saves::SlotSummary;
use rc_proto::{GameReadiness, PartnerBody};

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
fn a_guest_only_accepts_a_start_while_connected_ready_and_not_started() {
    let yes = ready(true, LOBBY_MODE_NEW_GAME);
    let no = ready(false, LOBBY_MODE_NEW_GAME);
    assert!(guest_accepts_start(true, true, false, yes));
    assert!(!guest_accepts_start(false, true, false, yes));
    assert!(!guest_accepts_start(true, true, true, yes));
    assert!(!guest_accepts_start(true, true, false, no));
    assert!(!guest_accepts_start(true, false, false, yes));
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

#[test]
fn a_guest_waits_while_the_hosts_game_is_not_linked() {
    let mut waiting = GuestWaiting::new(0);
    assert_eq!(
        waiting.step(GameReadiness::NotLinked, 10.0),
        GuestWaitStep::KeepWaiting
    );
    assert_eq!(
        waiting.step(GameReadiness::NotLinked, 50.0),
        GuestWaitStep::KeepWaiting
    );
}

#[test]
fn a_guest_launches_once_the_hosts_game_is_loading_or_later() {
    for readiness in [GameReadiness::Loading, GameReadiness::InPlay] {
        let mut waiting = GuestWaiting::new(1);
        assert_eq!(waiting.step(readiness, 5.0), GuestWaitStep::LaunchNow);
    }
}

#[test]
fn a_guest_gives_up_after_the_wait_limit() {
    let mut waiting = GuestWaiting::new(0);
    waiting.step(GameReadiness::NotLinked, 100.0);
    assert_eq!(
        waiting.step(
            GameReadiness::NotLinked,
            100.0 + GUEST_WAIT_FOR_HOST_GAME_SECONDS - 0.5
        ),
        GuestWaitStep::KeepWaiting
    );
    assert_eq!(
        waiting.step(
            GameReadiness::NotLinked,
            100.0 + GUEST_WAIT_FOR_HOST_GAME_SECONDS
        ),
        GuestWaitStep::GiveUp
    );
}

#[test]
fn a_host_game_that_appears_at_the_limit_still_wins() {
    let mut waiting = GuestWaiting::new(0);
    waiting.step(GameReadiness::NotLinked, 0.0);
    assert_eq!(
        waiting.step(
            GameReadiness::Loading,
            GUEST_WAIT_FOR_HOST_GAME_SECONDS + 5.0
        ),
        GuestWaitStep::LaunchNow
    );
}

fn own_inputs() -> OwnGameInputs {
    OwnGameInputs {
        started: true,
        closed: false,
        waiting_for_host_game: false,
        own: GameReadiness::NotLinked,
        partner: GameReadiness::NotLinked,
    }
}

#[test]
fn a_game_not_yet_started_reads_not_started() {
    let inputs = OwnGameInputs {
        started: false,
        ..own_inputs()
    };
    assert_eq!(own_game_state(inputs), GameState::NotStarted);
}

#[test]
fn a_started_game_that_is_not_linked_yet_reads_starting() {
    assert_eq!(own_game_state(own_inputs()), GameState::Starting);
}

#[test]
fn a_loading_game_with_no_partner_game_waits_for_the_partner() {
    let inputs = OwnGameInputs {
        own: GameReadiness::Loading,
        ..own_inputs()
    };
    assert_eq!(own_game_state(inputs), GameState::WaitingForPartner);
    let partner_up = OwnGameInputs {
        partner: GameReadiness::Loading,
        ..inputs
    };
    assert_eq!(own_game_state(partner_up), GameState::Starting);
}

#[test]
fn a_guest_waiting_for_the_hosts_game_reads_waiting_for_partner() {
    let inputs = OwnGameInputs {
        waiting_for_host_game: true,
        ..own_inputs()
    };
    assert_eq!(own_game_state(inputs), GameState::WaitingForPartner);
}

#[test]
fn a_game_in_play_reads_in_play_and_a_closed_one_reads_closed() {
    let playing = OwnGameInputs {
        own: GameReadiness::InPlay,
        ..own_inputs()
    };
    assert_eq!(own_game_state(playing), GameState::InPlay);
    let closed = OwnGameInputs {
        closed: true,
        ..playing
    };
    assert_eq!(own_game_state(closed), GameState::Closed);
}

#[test]
fn the_partners_game_reads_from_its_readiness_and_whether_it_was_ever_linked() {
    assert_eq!(
        partner_game_state(GameReadiness::NotLinked, false),
        GameState::NotStarted
    );
    assert_eq!(
        partner_game_state(GameReadiness::Loading, false),
        GameState::Starting
    );
    assert_eq!(
        partner_game_state(GameReadiness::InPlay, true),
        GameState::InPlay
    );
    assert_eq!(
        partner_game_state(GameReadiness::NotLinked, true),
        GameState::Closed
    );
}

#[test]
fn hosting_alone_lists_only_the_host_without_a_picker() {
    let rows = player_rows(RowInputs {
        viewer_is_guest: false,
        own_name: "Ada",
        peer_name: None,
        guest_body: PartnerBody::Nier,
    });
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].role, PlayerRole::Host);
    assert!(rows[0].you);
    assert!(!rows[0].picker);
}

#[test]
fn the_host_sees_the_guests_pick_and_only_the_guest_gets_a_picker() {
    let host_view = player_rows(RowInputs {
        viewer_is_guest: false,
        own_name: "Ada",
        peer_name: Some("Bo"),
        guest_body: PartnerBody::PrologueCharacter,
    });
    assert_eq!(host_view.len(), 2);
    assert!(host_view.iter().all(|row| !row.picker));
    assert_eq!(host_view[1].role, PlayerRole::Guest);
    assert_eq!(host_view[1].body, PartnerBody::PrologueCharacter);
    assert_eq!(host_view[0].body, PartnerBody::Nier);

    let guest_view = player_rows(RowInputs {
        viewer_is_guest: true,
        own_name: "Bo",
        peer_name: Some("Ada"),
        guest_body: PartnerBody::PrologueCharacter,
    });
    assert_eq!(guest_view[0].role, PlayerRole::Host);
    assert_eq!(guest_view[0].name, "Ada");
    assert!(!guest_view[0].picker);
    assert!(guest_view[1].you && guest_view[1].picker);
}

#[test]
fn the_picker_choices_leave_kaine_out() {
    assert!(!picker_choices().contains(&PartnerBody::SeedKaineE));
    assert_eq!(
        picker_choices(),
        &[PartnerBody::Nier, PartnerBody::PrologueCharacter]
    );
}

#[test]
fn every_character_has_a_label_and_the_nier_variants_share_one() {
    use rc_launcher::lobby::body_label;
    use rc_proto::PartnerBody;
    assert_eq!(body_label(PartnerBody::SeedKaineE), "Kainé");
    assert_eq!(
        body_label(PartnerBody::SeedNierY),
        body_label(PartnerBody::Nier)
    );
}

#[test]
fn host_edits_the_co_op_settings() {
    assert!(host_options_editable(false));
}

#[test]
fn guest_sees_the_co_op_settings_read_only() {
    assert!(!host_options_editable(true));
}
