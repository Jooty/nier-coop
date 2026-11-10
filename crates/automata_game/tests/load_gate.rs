use automata_game::address::{
    LOAD_DONE_REQUEST_VALUE, LOADING_SCREEN_ANSWER_VALUE, SCENE_LOADER_FUNCTION,
    SCENE_LOADER_STAGE_VALUE,
};
use automata_game::exe::ExeVariant;
use automata_game::load_gate::{
    AloneReason, GateInputs, GateState, HIGHEST_ROUND_TRIP_ADDED_TO_THE_RELEASE_LEAD_MS,
    HoldReading, LONGEST_WAIT_FOR_A_NAMED_RELEASE_MS, LoadGate, LoadSignal, LoaderAddresses,
    PARTNER_READY_WAIT_LIMIT_MS, PartnerLoad, RELEASE_LEAD_MS, Release,
    SCENE_LOADER_ANSWER_COMPARE_OFFSET, SCENE_LOADER_CODE_CHECKED_LEN, SCENE_LOADER_FUNCTION_START,
    SCENE_LOADER_REQUEST_WRITE_OFFSET, SCENE_LOADER_STAGE_READ_OFFSET,
    loader_code_is_the_noted_one,
};
use automata_game::travel::JumpRole;

const UNIX_START_MS: u64 = 1_800_000_000_000;
const MAIN_STAGE_READ: [u8; 6] = [0x8B, 0x0D, 0x34, 0x78, 0x8C, 0x00];
const MAIN_ANSWER_COMPARE: [u8; 7] = [0x44, 0x39, 0x35, 0x2D, 0xE5, 0xC9, 0x00];
const MAIN_END_OF_STAGE_THREE: [u8; 20] = [
    0xC7, 0x05, 0x2C, 0xD9, 0xC9, 0x00, 0x01, 0x00, 0x00, 0x00, 0xC7, 0x05, 0xFA, 0x6B, 0x8C, 0x00,
    0x04, 0x00, 0x00, 0x00,
];
const COMPAT_STAGE_READ: [u8; 6] = [0x8B, 0x0D, 0xF0, 0x37, 0x8C, 0x00];
const COMPAT_ANSWER_COMPARE: [u8; 7] = [0x44, 0x39, 0x35, 0xDD, 0xA4, 0xC9, 0x00];
const COMPAT_END_OF_STAGE_THREE: [u8; 20] = [
    0xC7, 0x05, 0xDC, 0x98, 0xC9, 0x00, 0x01, 0x00, 0x00, 0x00, 0xC7, 0x05, 0xB6, 0x2B, 0x8C, 0x00,
    0x04, 0x00, 0x00, 0x00,
];

fn armed() -> LoadGate {
    LoadGate::new(true, true, true).0
}

fn waiting_since(now_ms: u64) -> LoadGate {
    let mut gate = armed();
    gate.load_done(now_ms);
    gate
}

fn inputs(now_ms: u64, role: JumpRole, partner: PartnerLoad) -> GateInputs {
    GateInputs {
        now_ms,
        now_unix_ms: UNIX_START_MS + now_ms,
        role,
        partner,
    }
}

fn partner_of(signal: LoadSignal) -> PartnerLoad {
    PartnerLoad {
        waiting: signal.waiting,
        in_play: false,
        release_at_unix_ms: signal.release_at_unix_ms,
        round_trip_ms: None,
    }
}

fn loader_code(stage_read: &[u8], answer_compare: &[u8], end_of_stage_three: &[u8]) -> Vec<u8> {
    let mut code = vec![0xCC_u8; SCENE_LOADER_CODE_CHECKED_LEN];
    code[..SCENE_LOADER_FUNCTION_START.len()].copy_from_slice(&SCENE_LOADER_FUNCTION_START);
    code[SCENE_LOADER_STAGE_READ_OFFSET..][..stage_read.len()].copy_from_slice(stage_read);
    code[SCENE_LOADER_ANSWER_COMPARE_OFFSET..][..answer_compare.len()]
        .copy_from_slice(answer_compare);
    code[SCENE_LOADER_REQUEST_WRITE_OFFSET..][..end_of_stage_three.len()]
        .copy_from_slice(end_of_stage_three);
    code
}

fn addresses_of(variant: ExeVariant) -> LoaderAddresses {
    LoaderAddresses {
        loader_function: SCENE_LOADER_FUNCTION.rva(variant),
        stage_value: SCENE_LOADER_STAGE_VALUE.rva(variant),
        load_done_request_value: LOAD_DONE_REQUEST_VALUE.rva(variant),
        loading_screen_answer_value: LOADING_SCREEN_ANSWER_VALUE.rva(variant),
    }
}

#[test]
fn both_ready_gives_one_release_time_on_both() {
    let mut host = waiting_since(1_000);
    let mut guest = waiting_since(3_000);
    // The guest hears nothing of a release yet and keeps waiting.
    let step = guest.step(&inputs(3_010, JumpRole::Guest, partner_of(host.signal())));
    assert!(!step.release);
    // The host hears the guest wait and names a moment a short time ahead.
    let step = host.step(&inputs(3_020, JumpRole::Host, partner_of(guest.signal())));
    assert!(!step.release);
    let named = host
        .signal()
        .release_at_unix_ms
        .expect("the host names a time");
    assert_eq!(named, UNIX_START_MS + 3_020 + RELEASE_LEAD_MS);
    // The guest hears it before the moment and waits for it too.
    let step = guest.step(&inputs(3_100, JumpRole::Guest, partner_of(host.signal())));
    assert!(!step.release);
    assert_eq!(guest.signal().release_at_unix_ms, Some(named));
    let before = 3_020 + RELEASE_LEAD_MS - 1;
    assert!(
        !host
            .step(&inputs(before, JumpRole::Host, partner_of(guest.signal())))
            .release
    );
    assert!(
        !guest
            .step(&inputs(before, JumpRole::Guest, partner_of(host.signal())))
            .release
    );
    let at = 3_020 + RELEASE_LEAD_MS;
    assert!(
        host.step(&inputs(at, JumpRole::Host, partner_of(guest.signal())))
            .release
    );
    assert!(
        guest
            .step(&inputs(at, JumpRole::Guest, partner_of(host.signal())))
            .release
    );
    let together = Some(Release::Together { at_unix_ms: named });
    assert_eq!(host.release(), together);
    assert_eq!(guest.release(), together);
    // A release is given once.
    assert!(
        !host
            .step(&inputs(at + 16, JumpRole::Host, partner_of(guest.signal())))
            .release
    );
}

#[test]
fn the_guest_never_names_a_time() {
    let mut guest = waiting_since(0);
    let host_waiting = PartnerLoad {
        waiting: true,
        ..PartnerLoad::default()
    };
    let step = guest.step(&inputs(100, JumpRole::Guest, host_waiting));
    assert!(!step.release);
    assert_eq!(guest.signal().release_at_unix_ms, None);
    assert!(guest.signal().waiting);
}

#[test]
fn the_round_trip_time_is_added_to_the_lead_up_to_a_cap() {
    let mut host = waiting_since(0);
    let slow_partner = PartnerLoad {
        waiting: true,
        round_trip_ms: Some(60_000),
        ..PartnerLoad::default()
    };
    host.step(&inputs(10, JumpRole::Host, slow_partner));
    assert_eq!(
        host.signal().release_at_unix_ms,
        Some(
            UNIX_START_MS + 10 + RELEASE_LEAD_MS + HIGHEST_ROUND_TRIP_ADDED_TO_THE_RELEASE_LEAD_MS
        )
    );
}

#[test]
fn no_partner_goes_on_alone_after_the_limit() {
    let mut gate = waiting_since(500);
    let nobody = PartnerLoad::default();
    let just_before = 500 + PARTNER_READY_WAIT_LIMIT_MS - 1;
    assert!(
        !gate
            .step(&inputs(just_before, JumpRole::Solo, nobody))
            .release
    );
    assert!(gate.is_holding());
    let step = gate.step(&inputs(just_before + 1, JumpRole::Solo, nobody));
    assert!(step.release);
    assert!(step.logs.iter().any(|line| line.contains("went on alone")));
    assert!(matches!(
        gate.release(),
        Some(Release::Alone {
            reason: AloneReason::NoPartnerLinked,
            ..
        })
    ));
    assert_eq!(
        gate.alone_note().as_deref(),
        Some("went on alone, no partner was linked in time")
    );
    assert_eq!(gate.holding_status(just_before + 2), None);
}

#[test]
fn a_late_partner_is_not_waited_for_beyond_the_limit() {
    let mut host = waiting_since(0);
    let still_loading = PartnerLoad::default();
    assert!(
        !host
            .step(&inputs(44_000, JumpRole::Host, still_loading))
            .release
    );
    let step = host.step(&inputs(
        PARTNER_READY_WAIT_LIMIT_MS,
        JumpRole::Host,
        still_loading,
    ));
    assert!(step.release);
    assert!(matches!(
        host.release(),
        Some(Release::Alone {
            reason: AloneReason::PartnerNotReadyInTime,
            ..
        })
    ));
    // The late guest hears the time the host left at and follows at once.
    let mut guest = waiting_since(70_000);
    let step = guest.step(&inputs(70_016, JumpRole::Guest, partner_of(host.signal())));
    assert!(step.release);
}

#[test]
fn a_dropped_link_goes_on_alone_after_the_limit() {
    let mut guest = waiting_since(0);
    let host_loading = PartnerLoad::default();
    assert!(
        !guest
            .step(&inputs(5_000, JumpRole::Guest, host_loading))
            .release
    );
    // The link drops: no role and nothing heard any more.
    assert!(
        !guest
            .step(&inputs(6_000, JumpRole::Solo, PartnerLoad::default()))
            .release
    );
    assert!(guest.is_holding());
    let step = guest.step(&inputs(
        PARTNER_READY_WAIT_LIMIT_MS,
        JumpRole::Solo,
        PartnerLoad::default(),
    ));
    assert!(step.release);
    assert!(matches!(
        guest.release(),
        Some(Release::Alone {
            reason: AloneReason::NoPartnerLinked,
            ..
        })
    ));
}

#[test]
fn a_named_release_is_kept_when_the_link_drops() {
    let mut host = waiting_since(0);
    let guest_waiting = PartnerLoad {
        waiting: true,
        ..PartnerLoad::default()
    };
    host.step(&inputs(100, JumpRole::Host, guest_waiting));
    let step = host.step(&inputs(
        100 + RELEASE_LEAD_MS,
        JumpRole::Solo,
        PartnerLoad::default(),
    ));
    assert!(step.release);
    assert!(matches!(host.release(), Some(Release::Together { .. })));
}

#[test]
fn a_release_message_that_arrives_late_is_obeyed_at_once() {
    let mut guest = waiting_since(0);
    let named_long_ago = PartnerLoad {
        waiting: false,
        in_play: false,
        release_at_unix_ms: Some(UNIX_START_MS + 2_000),
        round_trip_ms: None,
    };
    let step = guest.step(&inputs(9_000, JumpRole::Guest, named_long_ago));
    assert!(step.release);
    assert_eq!(
        guest.release(),
        Some(Release::Together {
            at_unix_ms: UNIX_START_MS + 2_000
        })
    );
    assert!(step.logs.iter().any(|line| line.contains("7000 ms ago")));
}

#[test]
fn a_release_time_far_ahead_on_this_clock_is_not_waited_for() {
    let mut guest = waiting_since(0);
    let clock_of_another_pc = PartnerLoad {
        waiting: true,
        in_play: false,
        release_at_unix_ms: Some(UNIX_START_MS + 3_600_000),
        round_trip_ms: None,
    };
    assert!(
        !guest
            .step(&inputs(1_000, JumpRole::Guest, clock_of_another_pc))
            .release
    );
    let step = guest.step(&inputs(
        1_000 + LONGEST_WAIT_FOR_A_NAMED_RELEASE_MS,
        JumpRole::Guest,
        clock_of_another_pc,
    ));
    assert!(step.release);
    assert!(step.logs.iter().any(|line| line.contains("clocks differ")));
}

#[test]
fn a_partner_already_in_the_world_is_not_waited_for() {
    let mut guest = waiting_since(0);
    let in_play = PartnerLoad {
        in_play: true,
        ..PartnerLoad::default()
    };
    let step = guest.step(&inputs(16, JumpRole::Guest, in_play));
    assert!(step.release);
    assert!(matches!(
        guest.release(),
        Some(Release::Alone {
            reason: AloneReason::PartnerAlreadyInPlay,
            ..
        })
    ));
}

#[test]
fn nothing_is_released_before_the_load_is_done() {
    let mut gate = armed();
    assert!(gate.wants_hold());
    assert!(!gate.is_holding());
    let named = PartnerLoad {
        waiting: true,
        release_at_unix_ms: Some(UNIX_START_MS),
        ..PartnerLoad::default()
    };
    let step = gate.step(&inputs(90_000, JumpRole::Guest, named));
    assert!(!step.release);
    assert_eq!(gate.state(), &GateState::Loading);
    assert_eq!(gate.signal(), LoadSignal::default());
}

#[test]
fn the_gate_is_off_without_the_setting_a_session_or_the_hold() {
    for (start_together, in_a_session, hold_installed) in [
        (false, true, true),
        (true, false, true),
        (true, true, false),
    ] {
        let (mut gate, logs) = LoadGate::new(start_together, in_a_session, hold_installed);
        assert!(!gate.wants_hold());
        assert_eq!(logs.len(), 1);
        assert!(gate.load_done(0).is_empty());
        assert!(
            !gate
                .step(&inputs(99_000, JumpRole::Host, PartnerLoad::default()))
                .release
        );
        assert_eq!(gate.holding_status(0), None);
        assert_eq!(gate.alone_note(), None);
    }
}

#[test]
fn a_hold_the_game_or_the_hard_limit_ended_counts_as_going_on_alone() {
    for (reading, reason) in [
        (HoldReading::EndedByTheGame, AloneReason::HoldEndedByTheGame),
        (
            HoldReading::EndedByTheHardLimit,
            AloneReason::HoldEndedByTheHardLimit,
        ),
    ] {
        let mut gate = waiting_since(0);
        let logs = gate.hold_ended(reading, UNIX_START_MS + 5);
        assert_eq!(logs.len(), 1);
        assert_eq!(
            gate.release(),
            Some(Release::Alone {
                reason,
                at_unix_ms: UNIX_START_MS + 5
            })
        );
        assert!(gate.hold_ended(reading, UNIX_START_MS + 6).is_empty());
    }
    let mut gate = waiting_since(0);
    assert!(gate.hold_ended(HoldReading::Held, UNIX_START_MS).is_empty());
    assert!(gate.is_holding());
}

#[test]
fn the_status_note_counts_the_wait_down() {
    let gate = waiting_since(1_000);
    assert_eq!(
        gate.holding_status(11_000).as_deref(),
        Some("start: load done, waiting for the partner (at most 35 s more)")
    );
}

#[test]
fn the_loader_code_of_both_exes_refers_to_the_noted_addresses() {
    let main = loader_code(
        &MAIN_STAGE_READ,
        &MAIN_ANSWER_COMPARE,
        &MAIN_END_OF_STAGE_THREE,
    );
    assert!(loader_code_is_the_noted_one(
        &main,
        addresses_of(ExeVariant::Main)
    ));
    let compat = loader_code(
        &COMPAT_STAGE_READ,
        &COMPAT_ANSWER_COMPARE,
        &COMPAT_END_OF_STAGE_THREE,
    );
    assert!(loader_code_is_the_noted_one(
        &compat,
        addresses_of(ExeVariant::Compat)
    ));
    // One exe's code never passes for the other's addresses.
    assert!(!loader_code_is_the_noted_one(
        &main,
        addresses_of(ExeVariant::Compat)
    ));
    assert!(!loader_code_is_the_noted_one(
        &compat,
        addresses_of(ExeVariant::Main)
    ));
}

#[test]
fn loader_code_that_differs_anywhere_checked_is_refused() {
    let good = loader_code(
        &MAIN_STAGE_READ,
        &MAIN_ANSWER_COMPARE,
        &MAIN_END_OF_STAGE_THREE,
    );
    let addresses = addresses_of(ExeVariant::Main);
    for changed_at in [
        0,
        SCENE_LOADER_STAGE_READ_OFFSET,
        SCENE_LOADER_STAGE_READ_OFFSET + 2,
        SCENE_LOADER_ANSWER_COMPARE_OFFSET + 3,
        SCENE_LOADER_REQUEST_WRITE_OFFSET + 2,
        SCENE_LOADER_REQUEST_WRITE_OFFSET + 6,
        SCENE_LOADER_REQUEST_WRITE_OFFSET + 12,
        SCENE_LOADER_REQUEST_WRITE_OFFSET + 16,
    ] {
        let mut code = good.clone();
        code[changed_at] ^= 0x10;
        assert!(
            !loader_code_is_the_noted_one(&code, addresses),
            "a change at {changed_at:#X} must be refused"
        );
    }
    assert!(!loader_code_is_the_noted_one(
        &good[..SCENE_LOADER_CODE_CHECKED_LEN - 1],
        addresses
    ));
}

#[test]
fn a_start_that_is_over_gives_a_wanted_hold_up_once() {
    let mut gate = armed();
    let logs = gate.start_is_over(UNIX_START_MS);
    assert_eq!(logs.len(), 1);
    assert!(!gate.wants_hold());
    assert!(matches!(
        gate.release(),
        Some(Release::Alone {
            reason: AloneReason::LoadEndedUnheld,
            ..
        })
    ));
    assert!(gate.start_is_over(UNIX_START_MS + 1).is_empty());
    let mut unused = LoadGate::unused();
    assert!(!unused.wants_hold());
    assert!(unused.start_is_over(UNIX_START_MS).is_empty());
}
