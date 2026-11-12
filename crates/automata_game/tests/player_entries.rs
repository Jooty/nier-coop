use automata_game::mode::SessionMode;
use automata_game::partner::{PlayerEntry, player_entries};

fn entry(slot: u8, name: &str) -> PlayerEntry {
    PlayerEntry {
        slot,
        name: name.to_owned(),
    }
}

#[test]
fn one_line_per_other_player_in_slot_order() {
    let others = [entry(3, "Cy"), entry(1, "Ann"), entry(2, "Bo")];
    assert_eq!(
        player_entries(SessionMode::FreeRoam, &others),
        vec![entry(1, "Ann"), entry(2, "Bo"), entry(3, "Cy")]
    );
    assert_eq!(
        player_entries(SessionMode::FreeRoam, &[entry(2, "Bo"), entry(2, "Bo")]),
        vec![entry(2, "Bo")]
    );
    assert!(player_entries(SessionMode::FreeRoam, &[]).is_empty());
}

#[test]
fn the_campaign_shows_its_one_partner_only() {
    let others = [entry(2, "Bo"), entry(1, "Ann")];
    assert_eq!(
        player_entries(SessionMode::Campaign, &others),
        vec![entry(1, "Ann")]
    );
}
