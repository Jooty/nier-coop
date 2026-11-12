use automata_game::address::{
    ENEMY_GENERATOR_VFTABLE, GENERATOR_LEAVE_TEST_FUNCTION, PLAYER_PLACE_FOR_AREA_TESTS_FUNCTION,
};
use automata_game::enemies::{
    GUEST_LEAVE_TEST_FOLLOWS_THE_HOST_FOR_MS_AFTER_ITS_LAST_WORD, LeaveOutcome, LeaveTestRule,
    LeaveWatch, MOST_LEAVE_ANSWERS_KEPT, leave_test_rule,
};
use automata_game::exe::ExeVariant;

const A_GENERATOR: u32 = 0x3D40_0D95;

#[test]
fn the_leave_test_addresses_are_the_ones_read_from_both_exes() {
    let expected = [
        (GENERATOR_LEAVE_TEST_FUNCTION, 0x34_FBA0, 0x34_9F80),
        (ENEMY_GENERATOR_VFTABLE, 0xC3_0068, 0xC2_6020),
        (PLAYER_PLACE_FOR_AREA_TESTS_FUNCTION, 0x7C_4880, 0x7B_C540),
    ];
    for (address, main, compat) in expected {
        assert_eq!(address.rva(ExeVariant::Main), main);
        assert_eq!(address.rva(ExeVariant::Compat), compat);
    }
}

#[test]
fn without_the_link_or_with_shared_enemies_not_on_the_test_is_the_games_own() {
    for is_host in [true, false] {
        assert_eq!(
            leave_test_rule(is_host, false, true, true, Some(0)),
            LeaveTestRule::TheGamesOwn
        );
        assert_eq!(
            leave_test_rule(is_host, true, false, true, Some(0)),
            LeaveTestRule::TheGamesOwn
        );
    }
}

#[test]
fn the_linked_host_keeps_a_run_while_either_player_is_in_the_area() {
    assert_eq!(
        leave_test_rule(true, true, true, false, None),
        LeaveTestRule::EitherPlayerKeepsTheRun
    );
}

#[test]
fn the_guest_leaves_it_to_the_host_only_while_its_generators_are_held_and_the_host_was_heard() {
    assert_eq!(
        leave_test_rule(false, true, true, true, Some(0)),
        LeaveTestRule::TheHostDecides
    );
    assert_eq!(
        leave_test_rule(
            false,
            true,
            true,
            true,
            Some(GUEST_LEAVE_TEST_FOLLOWS_THE_HOST_FOR_MS_AFTER_ITS_LAST_WORD)
        ),
        LeaveTestRule::TheHostDecides
    );
    assert_eq!(
        leave_test_rule(false, true, true, false, Some(0)),
        LeaveTestRule::TheGamesOwn
    );
}

#[test]
fn a_silent_host_gives_the_guest_its_own_leave_test_back() {
    assert_eq!(
        leave_test_rule(
            false,
            true,
            true,
            true,
            Some(GUEST_LEAVE_TEST_FOLLOWS_THE_HOST_FOR_MS_AFTER_ITS_LAST_WORD + 1)
        ),
        LeaveTestRule::TheGamesOwn
    );
    assert_eq!(
        leave_test_rule(false, true, true, true, None),
        LeaveTestRule::TheGamesOwn
    );
}

#[test]
fn the_games_own_rule_answers_what_the_game_said() {
    assert_eq!(
        LeaveTestRule::TheGamesOwn.answer(true, Some(false)),
        (true, LeaveOutcome::LeftByTheGamesOwnTest)
    );
    assert_eq!(
        LeaveTestRule::TheGamesOwn.answer(false, Some(true)),
        (false, LeaveOutcome::PlayerIn)
    );
}

#[test]
fn the_guest_walking_away_never_ends_a_run_while_the_host_decides() {
    assert_eq!(
        LeaveTestRule::TheHostDecides.answer(true, None),
        (false, LeaveOutcome::HeldForTheHost)
    );
    assert_eq!(
        LeaveTestRule::TheHostDecides.answer(false, None),
        (false, LeaveOutcome::PlayerIn)
    );
}

#[test]
fn the_host_ends_a_run_only_when_the_partner_body_is_out_of_the_area_too() {
    let rule = LeaveTestRule::EitherPlayerKeepsTheRun;
    assert_eq!(
        rule.answer(false, Some(true)),
        (false, LeaveOutcome::PlayerIn)
    );
    assert_eq!(
        rule.answer(true, Some(false)),
        (false, LeaveOutcome::KeptByThePartner)
    );
    assert_eq!(
        rule.answer(true, Some(true)),
        (true, LeaveOutcome::BothLeft)
    );
}

#[test]
fn the_host_without_a_partner_body_ends_a_run_as_the_game_would() {
    assert_eq!(
        LeaveTestRule::EitherPlayerKeepsTheRun.answer(true, None),
        (true, LeaveOutcome::LeftWithoutAPartnerBody)
    );
}

#[test]
fn every_rule_comes_back_from_its_byte_and_an_unknown_byte_is_the_games_own() {
    for rule in [
        LeaveTestRule::TheGamesOwn,
        LeaveTestRule::TheHostDecides,
        LeaveTestRule::EitherPlayerKeepsTheRun,
    ] {
        assert_eq!(LeaveTestRule::from_byte(rule.as_byte()), rule);
    }
    assert_eq!(LeaveTestRule::from_byte(0xFF), LeaveTestRule::TheGamesOwn);
}

#[test]
fn a_line_is_logged_only_when_a_generators_outcome_changes_to_or_from_the_mods() {
    let mut watch = LeaveWatch::default();
    assert_eq!(watch.note(A_GENERATOR, LeaveOutcome::PlayerIn), None);
    let held = watch
        .note(A_GENERATOR, LeaveOutcome::HeldForTheHost)
        .expect("a line");
    assert!(held.contains("0x3D400D95"), "{held}");
    assert!(held.contains("until the host says"), "{held}");
    assert_eq!(watch.note(A_GENERATOR, LeaveOutcome::HeldForTheHost), None);
    assert!(
        watch
            .note(A_GENERATOR, LeaveOutcome::PlayerIn)
            .expect("a line")
            .contains("back in")
    );
    assert_eq!(
        watch.note(A_GENERATOR, LeaveOutcome::LeftByTheGamesOwnTest),
        None
    );
    assert_eq!(watch.note(A_GENERATOR, LeaveOutcome::PlayerIn), None);
}

#[test]
fn the_host_logs_the_partner_keeping_a_run_and_both_players_leaving() {
    let mut watch = LeaveWatch::default();
    assert!(
        watch
            .note(A_GENERATOR, LeaveOutcome::KeptByThePartner)
            .expect("a line")
            .contains("partner body is in it")
    );
    assert!(
        watch
            .note(A_GENERATOR, LeaveOutcome::BothLeft)
            .expect("a line")
            .contains("both players are out")
    );
    assert!(
        watch
            .note(A_GENERATOR, LeaveOutcome::LeftWithoutAPartnerBody)
            .expect("a line")
            .contains("no partner body")
    );
}

#[test]
fn the_watch_forgets_everything_when_it_is_full_and_a_new_generator_comes() {
    let mut watch = LeaveWatch::default();
    for generator in 0..MOST_LEAVE_ANSWERS_KEPT as u32 {
        watch.note(generator, LeaveOutcome::HeldForTheHost);
    }
    watch.note(u32::MAX, LeaveOutcome::PlayerIn);
    assert!(watch.note(0, LeaveOutcome::HeldForTheHost).is_some());
}
