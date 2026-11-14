use automata_game::character::Character;
use automata_game::enemies::EnemyShare;
use automata_game::save_redirect::SaveRole;
use automata_game::start::route_lead;

#[test]
fn a_guest_that_shares_or_watches_the_enemies_takes_the_hosts_route() {
    for share in [EnemyShare::On, EnemyShare::Watch] {
        assert_eq!(
            route_lead(SaveRole::Guest, Character::NineS, Character::TwoB, share),
            Character::TwoB
        );
    }
}

#[test]
fn the_host_keeps_the_route_of_its_own_hero() {
    for share in [EnemyShare::On, EnemyShare::Watch, EnemyShare::Off] {
        assert_eq!(
            route_lead(SaveRole::Host, Character::TwoB, Character::NineS, share),
            Character::TwoB
        );
    }
}

#[test]
fn with_the_enemy_share_off_the_guest_keeps_its_own_route_as_before() {
    assert_eq!(
        route_lead(
            SaveRole::Guest,
            Character::NineS,
            Character::TwoB,
            EnemyShare::Off
        ),
        Character::NineS
    );
}
