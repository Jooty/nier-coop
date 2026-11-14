use automata_game::load_gate::{
    AloneReason, GateInputs, LINKED_PARTNER_SILENT_GIVE_UP_MS, LoadGate,
    PARTNER_READY_WAIT_LIMIT_MS, PartnerLoad, Release, StoryPlace, load_hold_waiting_line,
};
use automata_game::travel::JumpRole;

fn waiting_since(now_ms: u64) -> LoadGate {
    let mut gate = LoadGate::for_a_campaign_load();
    gate.load_done(now_ms);
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

fn silent() -> PartnerLoad {
    PartnerLoad {
        never_heard: true,
        ..PartnerLoad::default()
    }
}

fn heard_without_body() -> PartnerLoad {
    PartnerLoad {
        heard_at_ms: 900,
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
fn a_linked_partner_never_heard_is_left_after_the_silent_limit() {
    for role in [JumpRole::Host, JumpRole::Guest] {
        let mut gate = waiting_since(2_000);
        let before = 2_000 + LINKED_PARTNER_SILENT_GIVE_UP_MS - 1;
        assert!(!gate.step(&inputs(before, role, silent())).release);
        assert!(gate.is_holding());
        let step = gate.step(&inputs(before + 1, role, silent()));
        assert!(step.release);
        assert_eq!(
            alone_reason(&gate),
            Some(AloneReason::PartnerLinkedButSilent)
        );
        assert!(step.logs[0].contains("linked but no state message from it ever arrived"));
    }
}

#[test]
fn a_partner_whose_state_was_heard_is_waited_for_up_to_the_ready_limit() {
    let mut gate = waiting_since(0);
    let after_silent_limit = LINKED_PARTNER_SILENT_GIVE_UP_MS + 5_000;
    assert!(
        !gate
            .step(&inputs(
                after_silent_limit,
                JumpRole::Host,
                heard_without_body()
            ))
            .release
    );
    let step = gate.step(&inputs(
        PARTNER_READY_WAIT_LIMIT_MS,
        JumpRole::Host,
        heard_without_body(),
    ));
    assert!(step.release);
    assert_eq!(
        alone_reason(&gate),
        Some(AloneReason::PartnerNotReadyInTime)
    );
}

#[test]
fn nobody_linked_keeps_its_own_reason_and_limit() {
    let mut gate = waiting_since(0);
    assert!(
        !gate
            .step(&inputs(
                LINKED_PARTNER_SILENT_GIVE_UP_MS + 1,
                JumpRole::Solo,
                silent()
            ))
            .release
    );
    gate.step(&inputs(
        PARTNER_READY_WAIT_LIMIT_MS,
        JumpRole::Solo,
        silent(),
    ));
    assert_eq!(alone_reason(&gate), Some(AloneReason::NoPartnerLinked));
}

#[test]
fn the_silent_reason_has_its_text() {
    assert_eq!(
        AloneReason::PartnerLinkedButSilent.to_string(),
        "the partner's game is linked but no state message from it ever arrived (its launcher or game may be another build than this one)"
    );
}

#[test]
fn the_hold_line_tells_the_three_cases_apart() {
    let here = StoryPlace::default();
    let line = |linked, partner: &PartnerLoad| {
        load_hold_waiting_line(4_200, 5_000, linked, &here, partner, &here)
    };
    assert!(line(false, &PartnerLoad::default()).contains("the partner not linked at"));
    assert!(line(true, &silent()).contains("linked for 4 s, no state message ever heard"));
    assert!(
        line(true, &heard_without_body()).contains("loading (a body-less state heard 4100 ms ago)")
    );
}
