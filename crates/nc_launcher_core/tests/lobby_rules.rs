use nc_launcher_core::{
    AUTOMATA, CO_OP_CAMPAIGN, FREE_ROAM, FREE_ROAM_MODE, HIDE_AND_SEEK_MODE, LobbyOption,
    PlayerStatus, REPLICANT, RosterSeat, SessionOptions, StartInputs, current_mode,
    lobby_permissions, option_available, party_misfit_note, remove_offered, roster, rules_for_mode,
    start_allowed,
};

const UNKNOWN_MODE: u8 = 200;

fn inputs(others_ready: &[bool], max_players_of: nc_launcher_core::GameMode) -> StartInputs<'_> {
    StartInputs {
        connected: true,
        viewer_is_guest: false,
        already_started: false,
        own_ready: true,
        others_ready,
        rules: max_players_of.rules(),
    }
}

#[test]
fn a_game_with_one_mode_lists_one_mode() {
    assert_eq!(REPLICANT.modes, &[CO_OP_CAMPAIGN]);
    assert_eq!(
        current_mode(&REPLICANT, CO_OP_CAMPAIGN.rules()),
        Some(&CO_OP_CAMPAIGN)
    );
}

#[test]
fn the_second_game_offers_the_campaign_and_free_roam_and_two_planned_modes() {
    let pickable: Vec<&str> = AUTOMATA
        .modes
        .iter()
        .filter(|mode| mode.pickable())
        .map(|mode| mode.label)
        .collect();
    assert_eq!(pickable, ["Co-op campaign", "Free roam"]);
    assert_eq!(AUTOMATA.modes.len(), 4);
}

#[test]
fn the_campaign_holds_two_and_every_other_mode_sixteen() {
    for profile in [&REPLICANT, &AUTOMATA] {
        for mode in profile.modes {
            let expected = if mode.is_campaign() { 2 } else { 16 };
            assert_eq!(mode.max_players, expected, "{}", mode.label);
        }
    }
}

#[test]
fn picking_a_mode_gives_its_player_count() {
    let rules = rules_for_mode(&AUTOMATA, FREE_ROAM_MODE).expect("free roam is pickable");
    assert_eq!((rules.mode, rules.max_players), (FREE_ROAM_MODE, 16));
    let rules = rules_for_mode(&AUTOMATA, CO_OP_CAMPAIGN.id).expect("the campaign is pickable");
    assert_eq!(rules.max_players, 2);
}

#[test]
fn a_planned_or_unknown_mode_cannot_be_picked() {
    assert_eq!(rules_for_mode(&AUTOMATA, HIDE_AND_SEEK_MODE), None);
    assert_eq!(rules_for_mode(&AUTOMATA, UNKNOWN_MODE), None);
    assert_eq!(rules_for_mode(&REPLICANT, FREE_ROAM_MODE), None);
}

#[test]
fn the_campaign_starts_with_two_ready_players() {
    assert!(start_allowed(inputs(&[true], CO_OP_CAMPAIGN)));
    assert!(!start_allowed(inputs(&[false], CO_OP_CAMPAIGN)));
    assert!(!start_allowed(inputs(&[], CO_OP_CAMPAIGN)));
}

#[test]
fn the_campaign_cannot_start_with_three() {
    assert!(!start_allowed(inputs(&[true, true], CO_OP_CAMPAIGN)));
    assert!(start_allowed(inputs(&[true, true], FREE_ROAM)));
}

#[test]
fn free_roam_cannot_start_with_seventeen() {
    let sixteen_others = [true; 16];
    assert!(!start_allowed(inputs(&sixteen_others, FREE_ROAM)));
    assert!(start_allowed(inputs(&sixteen_others[..15], FREE_ROAM)));
}

#[test]
fn only_the_host_starts() {
    let mut guest = inputs(&[true], CO_OP_CAMPAIGN);
    guest.viewer_is_guest = true;
    assert!(!start_allowed(guest));
    let mut started = inputs(&[true], CO_OP_CAMPAIGN);
    started.already_started = true;
    assert!(!start_allowed(started));
}

#[test]
fn a_guest_cannot_change_the_mode_or_the_options() {
    let guest = lobby_permissions(true);
    assert!(!guest.change_mode);
    assert!(!guest.change_options);
    assert!(!guest.remove_members);
    let host = lobby_permissions(false);
    assert!(host.change_mode && host.change_options && host.remove_members);
}

#[test]
fn ping_and_distance_are_unavailable_when_names_are_off() {
    let names_off = SessionOptions {
        show_names: false,
        ..SessionOptions::default()
    };
    assert!(!option_available(&names_off, LobbyOption::Distance));
    assert!(!option_available(&names_off, LobbyOption::Ping));
    assert!(option_available(&names_off, LobbyOption::PartyFrame));
    let names_on = SessionOptions {
        show_names: true,
        ..SessionOptions::default()
    };
    assert!(option_available(&names_on, LobbyOption::Distance));
    assert!(option_available(&names_on, LobbyOption::Ping));
}

#[test]
fn only_the_host_is_offered_remove() {
    assert!(remove_offered(false, false));
    assert!(!remove_offered(false, true));
    assert!(!remove_offered(true, false));
    assert!(!remove_offered(true, true));
}

#[test]
fn a_party_that_does_not_fit_the_mode_is_told_why() {
    assert_eq!(
        party_misfit_note(3, CO_OP_CAMPAIGN.rules()).as_deref(),
        Some("This mode takes 2 players; 3 are in")
    );
    assert_eq!(party_misfit_note(2, CO_OP_CAMPAIGN.rules()), None);
    assert_eq!(party_misfit_note(16, FREE_ROAM.rules()), None);
}

#[test]
fn up_to_eight_players_fill_one_column_of_eight() {
    let board = roster(&[1, 2, 3, 4, 5], false);
    assert_eq!(board.columns, 1);
    assert_eq!(board.seats.len(), 8);
    assert_eq!(board.seats[0], RosterSeat::Own);
    assert_eq!(board.seats[5], RosterSeat::Member(4));
    assert_eq!(board.seats[6], RosterSeat::Open);
}

#[test]
fn a_ninth_player_opens_the_second_column() {
    let slots: Vec<u8> = (1..=8).collect();
    let board = roster(&slots, false);
    assert_eq!(board.columns, 2);
    assert_eq!(board.seats.len(), 16);
    assert_eq!(board.seats[8], RosterSeat::Member(7));
    assert_eq!(board.seats[9], RosterSeat::Open);
}

#[test]
fn a_guest_sits_at_the_lowest_free_slot() {
    let board = roster(&[2, 0, 3], true);
    assert_eq!(
        &board.seats[..4],
        &[
            RosterSeat::Member(1),
            RosterSeat::Own,
            RosterSeat::Member(0),
            RosterSeat::Member(2)
        ]
    );
}

#[test]
fn ready_and_in_game_are_fine_and_every_other_status_is_not() {
    assert!(PlayerStatus::Ready.is_fine());
    assert!(PlayerStatus::InGame.is_fine());
    assert!(!PlayerStatus::NotReady.is_fine());
    assert!(!PlayerStatus::StartingGame.is_fine());
    assert!(!PlayerStatus::GameClosed.is_fine());
}
