use std::path::Path;

use nc_launcher_core::{
    AUTOMATA, CO_OP_CAMPAIGN, FREE_ROAM, REPLICANT, campaign_character, current_map,
    map_banner_path, map_pick_offered, rules_with_map,
};

#[test]
fn the_second_games_free_roam_maps_are_the_chapter_candidates_of_distinct_places() {
    let maps: Vec<(&str, u8)> = AUTOMATA
        .free_roam_maps
        .iter()
        .map(|map| (map.id, map.start_index))
        .collect();
    assert_eq!(
        maps,
        [
            ("city-ruins", 0),
            ("desert", 1),
            ("amusement-park", 2),
            ("pascals-village", 3),
            ("flooded-city", 5)
        ]
    );
    assert!(REPLICANT.free_roam_maps.is_empty());
}

#[test]
fn only_a_mode_that_starts_on_a_map_has_one() {
    assert_eq!(current_map(&AUTOMATA, CO_OP_CAMPAIGN.rules()), None);
    assert_eq!(
        current_map(&AUTOMATA, FREE_ROAM.rules()).map(|map| map.id),
        Some("city-ruins")
    );
    assert_eq!(current_map(&REPLICANT, FREE_ROAM.rules()), None);
}

#[test]
fn the_host_picks_a_map_by_its_place_in_the_list() {
    let rules = rules_with_map(&AUTOMATA, FREE_ROAM.rules(), 4).expect("a fifth map");
    assert_eq!(rules.map, 4);
    assert_eq!(rules.mode, FREE_ROAM.rules().mode);
    assert_eq!(
        current_map(&AUTOMATA, rules).map(|map| (map.label, map.start_index)),
        Some(("Flooded City", 5))
    );
    assert_eq!(rules_with_map(&AUTOMATA, FREE_ROAM.rules(), 5), None);
    assert_eq!(rules_with_map(&AUTOMATA, CO_OP_CAMPAIGN.rules(), 1), None);
}

#[test]
fn an_unknown_map_number_from_an_older_host_falls_back_to_the_first_map() {
    let mut rules = FREE_ROAM.rules();
    rules.map = 200;
    assert_eq!(
        current_map(&AUTOMATA, rules).map(|map| map.id),
        Some("city-ruins")
    );
}

#[test]
fn only_the_host_is_offered_the_map_choice_and_only_in_free_roam() {
    assert!(map_pick_offered(&AUTOMATA, FREE_ROAM.rules(), false));
    assert!(!map_pick_offered(&AUTOMATA, FREE_ROAM.rules(), true));
    assert!(!map_pick_offered(&AUTOMATA, CO_OP_CAMPAIGN.rules(), false));
}

#[test]
fn a_banner_is_read_from_the_launchers_data_folder_by_game_and_map() {
    assert_eq!(
        map_banner_path(Path::new("/data/nier-together"), "automata", "desert"),
        Path::new("/data/nier-together/map-banners/automata/desert.png")
    );
}

#[test]
fn in_the_campaign_the_host_is_the_hero_and_the_guest_the_companion() {
    let id = |profile, is_guest| campaign_character(profile, is_guest).map(|c| c.id);
    assert_eq!(id(&AUTOMATA, false), Some("2b"));
    assert_eq!(id(&AUTOMATA, true), Some("9s"));
    assert_eq!(id(&REPLICANT, false), Some("nier"));
    assert_eq!(id(&REPLICANT, true), Some("kaine"));
}
