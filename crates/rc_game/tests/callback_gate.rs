use rc_game::callback_gate::{
    CALLBACK_GATE_AFTER_DIALOG_CLOSE_MS, CALLBACK_GATE_GUEST_GIVE_UP_MS,
    CALLBACK_GATE_HOST_GRACE_MS, CALLBACK_GATE_PAIRING_WINDOW_MS, CallbackGate,
    END_FOLLOW_HOST_FIRED_WINDOW_MS, EndArrival, EndDrop, EndFollow, GateRole, GateStep, LocalFire,
    PartnerFire, end_follow, is_end_callback, is_gated_callback,
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
        role: GateRole::Host,
        message_phase: 4,
        shared_phase: Some(4),
        local_phase: Some(4),
        menu_open: false,
        host_in_own_talk: false,
        host_fired_it_ms_ago: None,
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
        host_in_own_talk: true,
        ..host_arrival()
    };
    assert_eq!(end_follow(&menu), EndFollow::Drop(EndDrop::MenuOpen));
    assert_eq!(end_follow(&talk), EndFollow::Drop(EndDrop::HostInOwnTalk));
}

#[test]
fn a_guest_end_the_host_fired_itself_runs_only_once() {
    let recent = EndArrival {
        host_fired_it_ms_ago: Some(END_FOLLOW_HOST_FIRED_WINDOW_MS),
        ..host_arrival()
    };
    let long_ago = EndArrival {
        host_fired_it_ms_ago: Some(END_FOLLOW_HOST_FIRED_WINDOW_MS + 1),
        ..host_arrival()
    };
    assert_eq!(
        end_follow(&recent),
        EndFollow::Drop(EndDrop::HostFiredItItself)
    );
    assert_eq!(end_follow(&long_ago), EndFollow::Mirror);
}

#[test]
fn the_guest_never_runs_a_mirrored_end() {
    let guest = EndArrival {
        role: GateRole::Guest,
        ..host_arrival()
    };
    assert_eq!(end_follow(&guest), EndFollow::Drop(EndDrop::NotTheHost));
}
