use replicant_game::script_functions::{FillInReach, fill_in_reach, relayed_function};
use replicant_game::shared_sequence::{STEPS_ONLY_THE_HOSTS_GUIDE_REACHES, is_led_by_the_host};

const TOWN: &str = "A_DESERT_TOWN_01";

#[test]
fn the_guides_motion_end_that_kept_the_guests_controls_locked_is_led_by_the_host() {
    // The run: the host's MiniGame_End at 17:19:13.579 never ran on the guest, whose pause stayed
    // 0x480 until 17:20:55.189 and held every later step of the tour behind it.
    assert!(is_led_by_the_host(TOWN, "MiniGame_End"));
    assert_eq!(
        fill_in_reach(TOWN, "MiniGame_End"),
        FillInReach::CannotReachHere
    );
    let relayed = relayed_function(TOWN, "MiniGame_End").unwrap();
    assert!(!relayed.starts_scene);
}

#[test]
fn the_tour_steps_the_guide_starts_run_on_the_guest_at_once() {
    // The run: each of these waited 3000 ms on the guest for a copy its game never fired.
    for step in [
        "EID_1410_a0075g_01",
        "EID_1410_a0100g_03",
        "EID_1410_a0100g_05",
        "EID_1410_a0100g_05_01",
        "EID_1410_a0100g_15",
        "EID_1410_a0100g_15_01",
    ] {
        assert!(is_led_by_the_host(TOWN, step), "{step}");
        assert_eq!(
            fill_in_reach(TOWN, step),
            FillInReach::CannotReachHere,
            "{step}"
        );
    }
}

#[test]
fn starts_ends_and_the_skip_answers_keep_their_own_paths() {
    for name in [
        "EID_1410_a0100g_Start",
        "EID_1410_a0075g_End",
        "EID_1410_a0100g_16_GuideSkip_01",
        "EID_1410_a0100g_16_GuideSkip_02",
        "Put_Boat_04",
    ] {
        assert!(!is_led_by_the_host(TOWN, name), "{name}");
    }
    assert!(!is_led_by_the_host("A_DESERT_FIELD_01", "MiniGame_End"));
}

#[test]
fn the_table_is_sorted_for_its_lookup() {
    assert!(
        STEPS_ONLY_THE_HOSTS_GUIDE_REACHES
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    );
}
