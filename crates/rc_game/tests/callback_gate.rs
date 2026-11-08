use rc_game::callback_gate::{
    CALLBACK_GATE_AFTER_DIALOG_CLOSE_MS, CALLBACK_GATE_GUEST_GIVE_UP_MS,
    CALLBACK_GATE_HOST_GRACE_MS, CALLBACK_GATE_PAIRING_WINDOW_MS, CallbackGate,
    END_FOLLOW_WAIT_CAP_MS, EndArrival, EndDrop, EndFollow, GateRole, GateStep, LocalFire,
    MaskIdleTimer, PartnerFire, PartnerStartedCopies, RanEnds, WaitStep, end_follow, event_base,
    event_family, is_end_callback, is_event_talk, is_gated_callback, is_scene_talk, wait_step,
};
use rc_game::ui_scenes::PAUSE_MENU_CLOSE_MAX_REQUESTS_PER_SCENE_START;

const BOSS: &str = "Boss_Battle_Start";
const SCENE: &str = "EID_0050_a0044g_Start";

#[test]
fn scene_and_boss_starts_are_gated_but_ends_and_waves_are_not() {
    assert!(is_gated_callback(BOSS));
    assert!(is_gated_callback(SCENE));
    assert!(is_gated_callback("EID_0050_a0044g_Start2"));
    assert!(!is_gated_callback("EID_0050_a0040g_End"));
    assert!(!is_gated_callback("Put_Enemy_AL1_160_00"));
    assert!(!is_gated_callback("A0050_a0042g_CYCLOPS_PadShake"));
}

#[test]
fn the_guest_holds_a_start_the_host_has_not_fired() {
    let mut gate = CallbackGate::new();
    assert_eq!(gate.local_fire(GateRole::Guest, BOSS, 0), LocalFire::Hold);
    assert!(gate.holds_any());
}

#[test]
fn the_hosts_fire_releases_the_held_guest_start_once() {
    let mut gate = CallbackGate::new();
    gate.local_fire(GateRole::Guest, BOSS, 100);
    assert_eq!(
        gate.partner_fire(GateRole::Guest, BOSS, 600),
        PartnerFire::Release { waited_ms: 500 }
    );
    assert_eq!(
        gate.local_fire(GateRole::Guest, BOSS, 650),
        LocalFire::RunReleased
    );
    assert_eq!(gate.local_fire(GateRole::Guest, BOSS, 700), LocalFire::Hold);
}

#[test]
fn a_start_the_host_fired_first_runs_on_the_guest_at_once() {
    let mut gate = CallbackGate::new();
    assert_eq!(
        gate.partner_fire(GateRole::Guest, SCENE, 0),
        PartnerFire::FollowHost
    );
    assert_eq!(
        gate.local_fire(GateRole::Guest, SCENE, 1_000),
        LocalFire::RunAfterPartner
    );
    assert_eq!(
        gate.local_fire(GateRole::Guest, SCENE, 2_000),
        LocalFire::Hold
    );
}

#[test]
fn the_hosts_announcement_goes_stale() {
    let mut gate = CallbackGate::new();
    gate.partner_fire(GateRole::Guest, SCENE, 0);
    assert_eq!(
        gate.local_fire(GateRole::Guest, SCENE, CALLBACK_GATE_PAIRING_WINDOW_MS + 1),
        LocalFire::Hold
    );
}

#[test]
fn the_guest_gives_up_and_runs_its_held_start_after_a_long_wait() {
    let mut gate = CallbackGate::new();
    gate.local_fire(GateRole::Guest, BOSS, 0);
    assert!(
        gate.step(
            GateRole::Guest,
            false,
            false,
            CALLBACK_GATE_GUEST_GIVE_UP_MS - 1
        )
        .is_empty()
    );
    assert_eq!(
        gate.step(
            GateRole::Guest,
            false,
            false,
            CALLBACK_GATE_GUEST_GIVE_UP_MS
        ),
        vec![GateStep::GaveUp {
            name: BOSS.to_owned(),
            waited_ms: CALLBACK_GATE_GUEST_GIVE_UP_MS
        }]
    );
    assert_eq!(
        gate.local_fire(GateRole::Guest, BOSS, CALLBACK_GATE_GUEST_GIVE_UP_MS + 5),
        LocalFire::RunReleased
    );
}

#[test]
fn the_host_waits_for_its_own_fire_then_starts_the_guests_step() {
    let mut gate = CallbackGate::new();
    assert_eq!(
        gate.partner_fire(GateRole::Host, BOSS, 0),
        PartnerFire::HostWaits
    );
    assert!(
        gate.step(
            GateRole::Host,
            false,
            false,
            CALLBACK_GATE_HOST_GRACE_MS - 1
        )
        .is_empty()
    );
    assert_eq!(
        gate.step(GateRole::Host, false, false, CALLBACK_GATE_HOST_GRACE_MS),
        vec![GateStep::Inject {
            name: BOSS.to_owned(),
            waited_ms: CALLBACK_GATE_HOST_GRACE_MS
        }]
    );
    assert_eq!(
        gate.local_fire(GateRole::Host, BOSS, CALLBACK_GATE_HOST_GRACE_MS + 10),
        LocalFire::RunReleased
    );
}

#[test]
fn the_host_closes_an_open_prompt_before_it_starts_the_guests_step() {
    let mut gate = CallbackGate::new();
    gate.partner_fire(GateRole::Host, BOSS, 0);
    let at = CALLBACK_GATE_HOST_GRACE_MS;
    assert_eq!(
        gate.step(GateRole::Host, true, false, at),
        vec![GateStep::CloseDialog {
            name: BOSS.to_owned(),
            waited_ms: at
        }]
    );
    assert!(gate.step(GateRole::Host, true, false, at + 100).is_empty());
    let later = at + CALLBACK_GATE_AFTER_DIALOG_CLOSE_MS;
    assert_eq!(
        gate.step(GateRole::Host, true, false, later),
        vec![GateStep::Inject {
            name: BOSS.to_owned(),
            waited_ms: later
        }]
    );
}

#[test]
fn a_host_that_fires_by_itself_is_not_forced() {
    let mut gate = CallbackGate::new();
    gate.partner_fire(GateRole::Host, BOSS, 0);
    assert_eq!(
        gate.local_fire(GateRole::Host, BOSS, 1_000),
        LocalFire::RunFirst
    );
    assert!(gate.step(GateRole::Host, false, false, 60_000).is_empty());
}

#[test]
fn a_host_start_the_guest_already_announced_is_not_waited_for_twice() {
    let mut gate = CallbackGate::new();
    gate.local_fire(GateRole::Host, SCENE, 0);
    assert_eq!(
        gate.partner_fire(GateRole::Host, SCENE, 400),
        PartnerFire::HostFiredFirst
    );
    assert!(gate.step(GateRole::Host, false, false, 60_000).is_empty());
}

#[test]
fn leaving_the_play_releases_the_held_starts() {
    let mut gate = CallbackGate::new();
    gate.local_fire(GateRole::Guest, BOSS, 0);
    assert_eq!(gate.release_held(50), vec![BOSS.to_owned()]);
    assert_eq!(
        gate.local_fire(GateRole::Guest, BOSS, 60),
        LocalFire::RunReleased
    );
}

#[test]
fn a_step_allowed_by_hand_runs_without_a_hold() {
    let mut gate = CallbackGate::new();
    gate.allow_next(SCENE, 0);
    assert_eq!(
        gate.local_fire(GateRole::Guest, SCENE, 10),
        LocalFire::RunReleased
    );
}

#[test]
fn a_guest_away_from_the_host_runs_its_start_alone_without_holding() {
    let mut gate = CallbackGate::new();
    gate.note_partner_here(false);
    assert_eq!(
        gate.local_fire(GateRole::Guest, SCENE, 0),
        LocalFire::RunAlone
    );
    assert!(!gate.holds_any());
}

#[test]
fn the_host_does_not_wait_for_or_start_a_guest_fire_from_elsewhere() {
    let mut gate = CallbackGate::new();
    gate.note_partner_here(false);
    assert_eq!(
        gate.partner_fire(GateRole::Host, SCENE, 0),
        PartnerFire::Elsewhere
    );
    assert!(
        gate.step(GateRole::Host, false, false, CALLBACK_GATE_HOST_GRACE_MS)
            .is_empty()
    );
}

#[test]
fn a_guest_in_the_same_place_follows_the_hosts_event_start_but_not_its_boss_signal() {
    let mut gate = CallbackGate::new();
    assert_eq!(
        gate.partner_fire(GateRole::Guest, SCENE, 0),
        PartnerFire::FollowHost
    );
    assert_eq!(
        gate.partner_fire(GateRole::Guest, BOSS, 0),
        PartnerFire::Noted
    );
}

#[test]
fn a_guest_elsewhere_only_notes_the_hosts_event_start() {
    let mut gate = CallbackGate::new();
    gate.note_partner_here(false);
    assert_eq!(
        gate.partner_fire(GateRole::Guest, SCENE, 0),
        PartnerFire::Noted
    );
}

#[test]
fn a_followed_start_runs_once_whichever_copy_pops_first() {
    let mut gate = CallbackGate::new();
    gate.partner_fire(GateRole::Guest, SCENE, 0);
    assert_eq!(
        gate.local_fire(GateRole::Guest, SCENE, 10),
        LocalFire::RunAfterPartner
    );
    assert!(!gate.holds_any());
}

#[test]
fn a_held_guest_start_runs_from_the_hosts_start_and_the_host_still_steps_in_after_its_grace() {
    let mut gate = CallbackGate::new();
    assert_eq!(gate.local_fire(GateRole::Guest, SCENE, 0), LocalFire::Hold);
    assert_eq!(
        gate.partner_fire(GateRole::Guest, SCENE, 3_041),
        PartnerFire::Release { waited_ms: 3_041 }
    );
    let mut host = CallbackGate::new();
    assert_eq!(
        host.partner_fire(GateRole::Host, SCENE, 0),
        PartnerFire::HostWaits
    );
    assert_eq!(
        host.step(GateRole::Host, false, false, CALLBACK_GATE_HOST_GRACE_MS),
        vec![GateStep::Inject {
            name: SCENE.to_owned(),
            waited_ms: CALLBACK_GATE_HOST_GRACE_MS
        }]
    );
}

#[test]
fn the_host_in_the_pause_menu_closes_it_at_once_before_the_guests_step_starts() {
    let mut gate = CallbackGate::new();
    gate.partner_fire(GateRole::Host, BOSS, 0);
    assert_eq!(
        gate.step(GateRole::Host, false, true, 10),
        vec![GateStep::ClosePauseMenu {
            name: BOSS.to_owned(),
            waited_ms: 10
        }]
    );
    assert!(gate.step(GateRole::Host, false, false, 20).is_empty());
    let settled = 10 + CALLBACK_GATE_AFTER_DIALOG_CLOSE_MS;
    assert_eq!(
        gate.step(GateRole::Host, false, false, settled),
        vec![GateStep::Inject {
            name: BOSS.to_owned(),
            waited_ms: settled
        }]
    );
}

#[test]
fn a_pause_menu_that_stays_open_is_closed_only_twice_per_start() {
    let mut gate = CallbackGate::new();
    gate.partner_fire(GateRole::Host, BOSS, 0);
    let mut closes = 0;
    for tick in 0..20 {
        let now = tick * CALLBACK_GATE_AFTER_DIALOG_CLOSE_MS;
        closes += gate
            .step(GateRole::Host, false, true, now)
            .iter()
            .filter(|step| matches!(step, GateStep::ClosePauseMenu { .. }))
            .count();
    }
    assert_eq!(
        closes,
        usize::from(PAUSE_MENU_CLOSE_MAX_REQUESTS_PER_SCENE_START)
    );
}

const TALK_END: &str = "EID_0100_a0039g_01_End";

fn host_arrival() -> EndArrival {
    EndArrival {
        message_phase: 4,
        shared_phase: Some(4),
        local_phase: Some(4),
        menu_open: false,
        local_in_own_talk: false,
        control_mask_settled: true,
        already_ran_here: false,
        own_copy_of_same_event_running: false,
    }
}

#[test]
fn only_event_ends_are_mirrored_names() {
    assert!(is_end_callback(TALK_END));
    assert!(is_end_callback("EID_0050_a0040g_End2"));
    assert!(!is_end_callback("EID_0100_a0039g_01_Start"));
    assert!(!is_end_callback("Put_Enemy_AL1_160_00"));
}

#[test]
fn the_hosts_game_runs_a_guest_end_when_nothing_blocks_it() {
    assert_eq!(end_follow(&host_arrival()), EndFollow::Mirror);
}

#[test]
fn a_guest_end_is_dropped_in_another_phase() {
    let other_message = EndArrival {
        message_phase: 5,
        ..host_arrival()
    };
    let other_local = EndArrival {
        local_phase: Some(5),
        ..host_arrival()
    };
    let unknown_local = EndArrival {
        local_phase: None,
        ..host_arrival()
    };
    for arrival in [other_message, other_local, unknown_local] {
        assert_eq!(end_follow(&arrival), EndFollow::Drop(EndDrop::OtherPhase));
    }
}

#[test]
fn a_guest_end_is_dropped_while_a_menu_is_open_or_the_host_is_in_its_own_talk() {
    let menu = EndArrival {
        menu_open: true,
        ..host_arrival()
    };
    let talk = EndArrival {
        local_in_own_talk: true,
        ..host_arrival()
    };
    assert_eq!(end_follow(&menu), EndFollow::Drop(EndDrop::MenuOpen));
    assert_eq!(end_follow(&talk), EndFollow::Drop(EndDrop::OwnTalkOpen));
}

#[test]
fn an_end_this_game_already_ran_is_not_run_again() {
    let ran = EndArrival {
        already_ran_here: true,
        ..host_arrival()
    };
    assert_eq!(end_follow(&ran), EndFollow::Drop(EndDrop::AlreadyRanHere));
}

#[test]
fn the_guests_game_runs_a_host_end_when_nothing_blocks_it() {
    assert_eq!(end_follow(&host_arrival()), EndFollow::Mirror);
    let menu = EndArrival {
        menu_open: true,
        ..host_arrival()
    };
    let talk = EndArrival {
        local_in_own_talk: true,
        ..host_arrival()
    };
    for arrival in [menu, talk] {
        assert_eq!(
            wait_step(0, 1_000, end_follow(&arrival)),
            WaitStep::KeepWaiting
        );
    }
}

#[test]
fn ran_ends_are_remembered_for_the_phase_not_for_a_time() {
    let mut ran = RanEnds::new();
    ran.note(TALK_END, 4);
    assert!(ran.contains(TALK_END, 4));
    assert!(!ran.contains("EID_0100_a0040g_End", 4));
    assert!(!ran.contains(TALK_END, 5));
    ran.note("EID_0100_a0040g_End", 5);
    assert!(!ran.contains(TALK_END, 4));
    assert!(ran.contains("EID_0100_a0040g_End", 5));
    ran.reset();
    assert!(!ran.contains("EID_0100_a0040g_End", 5));
}

#[test]
fn a_scene_talk_start_is_a_talk_and_an_event_start_is_not() {
    assert!(is_scene_talk("SEID_0100_S0010_a0020_Start"));
    assert!(is_scene_talk("SEID_0100_S0010_a0030_Start"));
    assert!(!is_scene_talk("EID_0100_a0010_Start"));
    assert!(!is_scene_talk("SEID_0100_S0010_a0020_End"));
    assert!(!is_scene_talk("Boss_Battle_Start"));
}

#[test]
fn a_talk_name_has_a_base_without_its_suffix() {
    assert_eq!(
        event_base("SEID_0100_S0010_a0020_End"),
        "SEID_0100_S0010_a0020"
    );
    assert_eq!(
        event_base("SEID_0100_S0010_a0030_Start"),
        "SEID_0100_S0010_a0030"
    );
}

#[test]
fn a_blocked_guest_end_waits_for_a_menu_or_the_hosts_own_talk() {
    let menu = EndArrival {
        menu_open: true,
        ..host_arrival()
    };
    let talk = EndArrival {
        local_in_own_talk: true,
        ..host_arrival()
    };
    for arrival in [menu, talk] {
        assert_eq!(
            wait_step(1_000, 5_000, end_follow(&arrival)),
            WaitStep::KeepWaiting
        );
    }
}

#[test]
fn a_waiting_guest_end_runs_when_the_block_ends() {
    assert_eq!(
        wait_step(1_000, 7_500, end_follow(&host_arrival())),
        WaitStep::Run { waited_ms: 6_500 }
    );
}

#[test]
fn a_waiting_guest_end_is_dropped_at_the_cap() {
    let menu = EndArrival {
        menu_open: true,
        ..host_arrival()
    };
    assert_eq!(
        wait_step(0, END_FOLLOW_WAIT_CAP_MS - 1, end_follow(&menu)),
        WaitStep::KeepWaiting
    );
    assert_eq!(
        wait_step(0, END_FOLLOW_WAIT_CAP_MS, end_follow(&menu)),
        WaitStep::GiveUp {
            waited_ms: END_FOLLOW_WAIT_CAP_MS
        }
    );
}

#[test]
fn a_waiting_guest_end_the_host_fired_itself_is_not_run_twice() {
    let fired = EndArrival {
        already_ran_here: true,
        ..host_arrival()
    };
    assert_eq!(
        wait_step(0, 10_000, end_follow(&fired)),
        WaitStep::Drop(EndDrop::AlreadyRanHere)
    );
    let other_phase = EndArrival {
        local_phase: Some(9),
        ..host_arrival()
    };
    assert_eq!(
        wait_step(0, 10_000, end_follow(&other_phase)),
        WaitStep::Drop(EndDrop::OtherPhase)
    );
}

#[test]
fn a_copy_started_from_the_partner_is_known_until_its_end_fires_here() {
    let mut copies = PartnerStartedCopies::new();
    copies.note("EID_0050_a0044g_Start", 3);
    assert!(copies.contains("EID_0050_a0044g_End", 3));
    assert!(copies.take("EID_0050_a0044g_End", 3));
    assert!(!copies.contains("EID_0050_a0044g_End", 3));
}

#[test]
fn a_copy_started_from_the_partner_has_no_time_window_only_a_phase() {
    let mut copies = PartnerStartedCopies::new();
    copies.note("EID_0200_a0040g_Talk", 23);
    assert!(copies.contains("EID_0200_a0040g_02_End", 23));
    assert!(copies.contains("EID_0200_a0040g_02_End", 23));
    assert!(!copies.contains("EID_0200_a0040g_02_End", 24));
    assert!(!copies.contains("EID_0200_a0040g_02_End", 23));
}

#[test]
fn run27_shrine_boss_talk_matches_its_end_but_not_the_next_event() {
    let mut copies = PartnerStartedCopies::new();
    copies.note("EID_0200_a0040g_Talk", 23);
    assert!(!copies.contains("EID_0200_a0050_End", 23));
    assert!(copies.contains("EID_0200_a0040g_02_End", 23));
    assert_eq!(event_family("EID_0200_a0040g_02_End"), "EID_0200_a0040g");
    assert_eq!(event_family("EID_0200_a0040g_Talk"), "EID_0200_a0040g");
    assert_eq!(
        event_family("SEID_0100_S0010_a0050_End01"),
        "SEID_0100_S0010_a0050"
    );
    assert!(is_event_talk("EID_0200_a0040g_Talk"));
    assert!(!is_event_talk("EID_0200_a0040g_02_End"));
}

#[test]
fn an_end_whose_event_my_own_copy_is_running_does_not_wait_for_the_mask() {
    let held = EndArrival {
        control_mask_settled: false,
        menu_open: true,
        own_copy_of_same_event_running: true,
        ..host_arrival()
    };
    assert_eq!(end_follow(&held), EndFollow::Mirror);
    let ran = EndArrival {
        already_ran_here: true,
        ..held
    };
    assert_eq!(end_follow(&ran), EndFollow::Drop(EndDrop::AlreadyRanHere));
    let other_phase = EndArrival {
        local_phase: Some(9),
        ..held
    };
    assert_eq!(
        end_follow(&other_phase),
        EndFollow::Drop(EndDrop::OtherPhase)
    );
}

#[test]
fn two_ends_of_one_talk_started_from_the_partner_are_each_known_once() {
    let mut copies = PartnerStartedCopies::new();
    copies.note("SEID_0100_S0010_a0050_End01", 4);
    copies.note("SEID_0100_S0010_a0050_End02", 4);
    assert!(copies.take("SEID_0100_S0010_a0050_End01", 4));
    assert!(copies.take("SEID_0100_S0010_a0050_End02", 4));
    assert!(!copies.take("SEID_0100_S0010_a0050_End02", 4));
}

#[test]
fn a_partner_end_waits_while_the_control_mask_is_not_idle() {
    let busy = EndArrival {
        control_mask_settled: false,
        ..host_arrival()
    };
    assert_eq!(end_follow(&busy), EndFollow::Drop(EndDrop::ControlMaskBusy));
    assert_eq!(
        wait_step(0, 5_000, end_follow(&busy)),
        WaitStep::KeepWaiting
    );
    assert_eq!(
        wait_step(0, END_FOLLOW_WAIT_CAP_MS, end_follow(&busy)),
        WaitStep::GiveUp {
            waited_ms: END_FOLLOW_WAIT_CAP_MS
        }
    );
}

#[test]
fn the_control_mask_counts_as_settled_after_half_a_second_of_idle() {
    let mut timer = MaskIdleTimer::new();
    assert!(!timer.observe(true, 1_000));
    assert!(!timer.observe(true, 1_499));
    assert!(timer.observe(true, 1_500));
    assert!(!timer.observe(false, 1_600));
    assert!(!timer.observe(true, 1_700));
    assert!(timer.observe(true, 2_200));
}
