use automata_game::load_gate::StoryPlace;
use automata_game::load_gate::{
    AloneReason, GateInputs, GateState, LoadGate, PARTNER_IN_ANOTHER_PLACE_WAIT_LIMIT_MS,
    PARTNER_READY_WAIT_LIMIT_MS, PartnerLoad, PlaceRelation, Release, alone_places_line,
    load_hold_waiting_line,
};
use automata_game::travel::JumpRole;

fn waiting_since(now_ms: u64) -> LoadGate {
    let mut gate = LoadGate::for_a_campaign_load();
    gate.load_done(now_ms);
    gate.step(&inputs(now_ms, JumpRole::Solo, PartnerLoad::default()));
    gate
}

fn inputs(now_ms: u64, role: JumpRole, partner: PartnerLoad) -> GateInputs {
    GateInputs {
        now_ms,
        role,
        loading_screen_ready: true,
        partner,
    }
}

fn partner(waiting: bool, in_play: bool, place: PlaceRelation) -> PartnerLoad {
    PartnerLoad {
        waiting,
        in_play,
        place,
        ..PartnerLoad::default()
    }
}

fn alone_reason(gate: &LoadGate) -> Option<AloneReason> {
    match gate.release() {
        Some(Release::Alone { reason, .. }) => Some(reason),
        _ => None,
    }
}

#[test]
fn a_waiting_partner_in_the_same_place_is_released_together_with_the_host_naming_the_moment() {
    let mut host = waiting_since(0);
    let step = host.step(&inputs(
        100,
        JumpRole::Host,
        partner(true, false, PlaceRelation::Same),
    ));
    assert!(!step.release);
    assert!(matches!(host.state(), GateState::ReleaseNamed { .. }));
}

#[test]
fn a_waiting_partner_of_unknown_place_is_treated_as_the_same_place() {
    let mut host = waiting_since(0);
    host.step(&inputs(
        100,
        JumpRole::Host,
        partner(true, false, PlaceRelation::Unknown),
    ));
    assert!(matches!(host.state(), GateState::ReleaseNamed { .. }));
}

#[test]
fn a_waiting_partner_in_another_place_is_left_at_once() {
    for role in [JumpRole::Host, JumpRole::Guest] {
        let mut gate = waiting_since(0);
        let step = gate.step(&inputs(
            100,
            role,
            partner(true, false, PlaceRelation::Other),
        ));
        assert!(step.release);
        assert_eq!(
            alone_reason(&gate),
            Some(AloneReason::PartnerLoadedAnotherPlace)
        );
    }
}

#[test]
fn a_partner_in_play_in_the_same_place_is_left_at_once() {
    let mut gate = waiting_since(0);
    let step = gate.step(&inputs(
        100,
        JumpRole::Guest,
        partner(false, true, PlaceRelation::Same),
    ));
    assert!(step.release);
    assert_eq!(alone_reason(&gate), Some(AloneReason::PartnerAlreadyInPlay));
}

#[test]
fn a_partner_in_play_in_another_place_is_waited_for_up_to_the_limit_from_the_start_of_the_wait() {
    let mut gate = waiting_since(1_000);
    let in_another_place = partner(false, true, PlaceRelation::Other);
    let just_before = 1_000 + PARTNER_IN_ANOTHER_PLACE_WAIT_LIMIT_MS - 1;
    for now_ms in [1_100, 5_000, just_before] {
        assert!(
            !gate
                .step(&inputs(now_ms, JumpRole::Guest, in_another_place))
                .release
        );
    }
    assert!(gate.is_holding());
    let step = gate.step(&inputs(
        1_000 + PARTNER_IN_ANOTHER_PLACE_WAIT_LIMIT_MS,
        JumpRole::Guest,
        in_another_place,
    ));
    assert!(step.release);
    assert_eq!(
        alone_reason(&gate),
        Some(AloneReason::PartnerStayedInAnotherPlace)
    );
}

#[test]
fn a_partner_in_another_place_that_comes_to_the_same_place_is_released_together() {
    let mut host = waiting_since(0);
    assert!(
        !host
            .step(&inputs(
                100,
                JumpRole::Host,
                partner(false, true, PlaceRelation::Other)
            ))
            .release
    );
    host.step(&inputs(
        200,
        JumpRole::Host,
        partner(true, false, PlaceRelation::Same),
    ));
    assert!(matches!(host.state(), GateState::ReleaseNamed { .. }));
}

#[test]
fn a_loading_or_silent_partner_is_waited_for_up_to_the_ready_limit() {
    for place in [PlaceRelation::Unknown, PlaceRelation::Other] {
        let mut gate = waiting_since(0);
        let loading = partner(false, false, place);
        assert!(
            !gate
                .step(&inputs(
                    PARTNER_READY_WAIT_LIMIT_MS - 1,
                    JumpRole::Guest,
                    loading
                ))
                .release
        );
        assert!(
            gate.step(&inputs(
                PARTNER_READY_WAIT_LIMIT_MS,
                JumpRole::Guest,
                loading
            ))
            .release
        );
        assert_eq!(
            alone_reason(&gate),
            Some(AloneReason::PartnerNotReadyInTime)
        );
    }
}

#[test]
fn a_release_moment_counts_only_from_a_partner_in_the_same_place() {
    let named = |place| PartnerLoad {
        waiting: true,
        release_in_ms: Some(-90_000),
        heard_at_ms: 90_000,
        place,
        ..PartnerLoad::default()
    };
    let mut other = waiting_since(89_000);
    other.step(&inputs(
        90_000,
        JumpRole::Guest,
        named(PlaceRelation::Other),
    ));
    assert_eq!(
        alone_reason(&other),
        Some(AloneReason::PartnerLoadedAnotherPlace)
    );
    let mut same = waiting_since(89_000);
    same.step(&inputs(90_000, JumpRole::Guest, named(PlaceRelation::Same)));
    assert_eq!(
        alone_reason(&same),
        Some(AloneReason::PartnerLeftItsLoadingScreenEarlier)
    );
}

#[test]
fn a_release_named_by_a_partner_that_then_moves_to_another_place_is_not_taken_over() {
    let mut guest = waiting_since(0);
    let named_same = PartnerLoad {
        waiting: true,
        release_in_ms: Some(400),
        heard_at_ms: 100,
        place: PlaceRelation::Same,
        ..PartnerLoad::default()
    };
    guest.step(&inputs(100, JumpRole::Guest, named_same));
    let named_other = PartnerLoad {
        place: PlaceRelation::Other,
        release_in_ms: Some(-10_000),
        ..named_same
    };
    assert!(
        !guest
            .step(&inputs(200, JumpRole::Guest, named_other))
            .release
    );
    assert_eq!(guest.signal().release_at_ms, Some(500));
}

#[test]
fn a_link_that_went_down_ends_a_wanted_hold_at_once_and_only_then() {
    let mut loading = LoadGate::for_a_campaign_load();
    let logs = loading.link_went_down(40);
    assert_eq!(alone_reason(&loading), Some(AloneReason::LinkWentDown));
    assert_eq!(logs.len(), 1);
    let mut waiting = waiting_since(0);
    waiting.link_went_down(40);
    assert_eq!(alone_reason(&waiting), Some(AloneReason::LinkWentDown));
    let mut unused = LoadGate::unused();
    assert!(unused.link_went_down(40).is_empty());
    assert_eq!(unused.release(), None);
}

#[test]
fn only_the_two_place_reasons_are_about_another_place() {
    assert!(AloneReason::PartnerLoadedAnotherPlace.is_about_another_place());
    assert!(AloneReason::PartnerStayedInAnotherPlace.is_about_another_place());
    assert!(!AloneReason::PartnerAlreadyInPlay.is_about_another_place());
    assert!(!AloneReason::LinkWentDown.is_about_another_place());
}

#[test]
fn the_places_lines_name_both_places() {
    let here = StoryPlace::new(0x100, "00_30_A_Place");
    let there = StoryPlace::new(0x200, "10_00_B_Place");
    let alone = alone_places_line(&here, &there);
    assert!(alone.contains("0x100 00_30_A_Place"));
    assert!(alone.contains("0x200 10_00_B_Place"));
    let waiting = load_hold_waiting_line(
        3_000,
        &here,
        &partner(true, false, PlaceRelation::Other),
        &StoryPlace::default(),
    );
    assert!(waiting.contains("3000 ms"));
    assert!(waiting.contains("0x100 00_30_A_Place"));
    assert!(waiting.contains("load done and waiting"));
    assert!(waiting.contains("0x0 (no name)"));
}
