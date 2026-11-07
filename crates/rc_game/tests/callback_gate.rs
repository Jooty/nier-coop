use rc_game::callback_gate::{
    CALLBACK_GATE_AFTER_DIALOG_CLOSE_MS, CALLBACK_GATE_GUEST_GIVE_UP_MS,
    CALLBACK_GATE_HOST_GRACE_MS, CALLBACK_GATE_PAIRING_WINDOW_MS, CallbackGate, GateRole, GateStep,
    LocalFire, PartnerFire, is_gated_callback,
};

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
        PartnerFire::Noted
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
        gate.step(GateRole::Guest, false, CALLBACK_GATE_GUEST_GIVE_UP_MS - 1)
            .is_empty()
    );
    assert_eq!(
        gate.step(GateRole::Guest, false, CALLBACK_GATE_GUEST_GIVE_UP_MS),
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
        gate.step(GateRole::Host, false, CALLBACK_GATE_HOST_GRACE_MS - 1)
            .is_empty()
    );
    assert_eq!(
        gate.step(GateRole::Host, false, CALLBACK_GATE_HOST_GRACE_MS),
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
        gate.step(GateRole::Host, true, at),
        vec![GateStep::CloseDialog {
            name: BOSS.to_owned(),
            waited_ms: at
        }]
    );
    assert!(gate.step(GateRole::Host, true, at + 100).is_empty());
    let later = at + CALLBACK_GATE_AFTER_DIALOG_CLOSE_MS;
    assert_eq!(
        gate.step(GateRole::Host, true, later),
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
    assert!(gate.step(GateRole::Host, false, 60_000).is_empty());
}

#[test]
fn a_host_start_the_guest_already_announced_is_not_waited_for_twice() {
    let mut gate = CallbackGate::new();
    gate.local_fire(GateRole::Host, SCENE, 0);
    assert_eq!(
        gate.partner_fire(GateRole::Host, SCENE, 400),
        PartnerFire::HostFiredFirst
    );
    assert!(gate.step(GateRole::Host, false, 60_000).is_empty());
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
