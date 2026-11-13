use replicant_game::script_functions::{
    FillInReach, FunctionArrival, FunctionArrivalFacts, FunctionDrop, FunctionsRan, fill_in_reach,
    function_arrival, relayed_function,
};
use replicant_game::shared_sequence::{STEPS_ONLY_THE_HOSTS_GUIDE_REACHES, is_led_by_the_host};

const TOWN: &str = "A_DESERT_TOWN_01";
const TOWN_PHASE: u16 = 7;

fn facts() -> FunctionArrivalFacts<'static> {
    FunctionArrivalFacts {
        enabled: true,
        local_in_play: true,
        local_phase: Some(TOWN_PHASE),
        call_phase: TOWN_PHASE,
        phase_label: TOWN,
        queue_is_empty: true,
    }
}

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

#[test]
fn the_hosts_copy_of_a_led_step_runs_and_a_second_copy_is_dropped() {
    let mut ran = FunctionsRan::new();
    assert_eq!(
        function_arrival(&facts(), "MiniGame_End", &ran, &[]),
        FunctionArrival::RunNow
    );
    ran.note(TOWN_PHASE, "MiniGame_End");
    assert_eq!(
        function_arrival(&facts(), "MiniGame_End", &ran, &[]),
        FunctionArrival::Drop(FunctionDrop::AlreadyRan)
    );
}

#[test]
fn a_skip_answer_never_runs_from_the_partners_shared_function_copy() {
    // The run: the host's GuideSkip_01 copy waited on the guest (17:20:40.379) beside the vote.
    let ran = FunctionsRan::new();
    for answer in [
        "EID_1410_a0100g_16_GuideSkip_01",
        "EID_1410_a0100g_16_GuideSkip_02",
    ] {
        assert_eq!(
            function_arrival(&facts(), answer, &ran, &[]),
            FunctionArrival::Drop(FunctionDrop::CarriedByTheSkipVote),
            "{answer}"
        );
    }
}
