use automata_game::mode::SessionMode;
use nc_wire::SessionRules;

fn rules(mode: u8) -> SessionRules {
    SessionRules {
        mode,
        max_players: 2,
        map: 0,
    }
}

#[test]
fn mode_zero_is_the_campaign() {
    assert_eq!(SessionMode::from_rules(rules(0)), SessionMode::Campaign);
    assert_eq!(SessionMode::from_rules(rules(1)), SessionMode::FreeRoam);
    assert_eq!(SessionMode::from_rules(rules(200)), SessionMode::FreeRoam);
    assert!(SessionMode::is_a_known_mode(0));
    assert!(SessionMode::is_a_known_mode(1));
    assert!(!SessionMode::is_a_known_mode(200));
}

#[test]
fn the_campaign_takes_exactly_two() {
    let range = SessionMode::Campaign.player_range();
    assert_eq!((*range.start(), *range.end()), (2, 2));
}

#[test]
fn free_roam_takes_two_to_sixteen() {
    let range = SessionMode::FreeRoam.player_range();
    assert_eq!((*range.start(), *range.end()), (2, 16));
}

#[test]
fn story_rules_are_off_outside_the_campaign() {
    assert!(SessionMode::Campaign.story_rules_apply());
    assert!(!SessionMode::FreeRoam.story_rules_apply());
}

#[test]
fn nobody_hosts_or_follows_a_jump_in_free_roam() {
    use automata_game::travel::JumpRole;
    assert_eq!(
        JumpRole::of_session(SessionMode::Campaign, true, Some(0)),
        JumpRole::Host
    );
    assert_eq!(
        JumpRole::of_session(SessionMode::Campaign, true, Some(1)),
        JumpRole::Guest
    );
    assert_eq!(
        JumpRole::of_session(SessionMode::FreeRoam, true, Some(1)),
        JumpRole::Solo
    );
}
