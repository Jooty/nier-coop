use automata_game::save_together::{
    GUEST_SAVE_NOT_TAKEN_AFTER_MS, GUEST_SAVE_WAIT_LIMIT_MS, GUEST_SAVE_WRITE_LIMIT_MS, GuestGates,
    MOST_HOST_SAVES_TOLD_PER_TICK, SAVE_WAITS_LOG_INTERVAL_MS, SaveMachine, SaveReadings,
};
use automata_game::travel::JumpRole;

fn free_and_idle() -> GuestGates {
    GuestGates {
        player_held_by: None,
        save_busy: Some(false),
        save_failed: Some(false),
    }
}

fn readings(role: JumpRole, now_ms: u64) -> SaveReadings {
    SaveReadings {
        now_ms,
        role,
        campaign_partner_present: true,
        local_in_play: true,
        partner_in_play: true,
        saves_asked_by_the_game: 0,
        host_saves_heard: Vec::new(),
        gates: None,
    }
}

fn guest_hears(machine: &mut SaveMachine, number: u32, now_ms: u64, gates: GuestGates) {
    let mut r = readings(JumpRole::Guest, now_ms);
    r.host_saves_heard = vec![number];
    r.gates = Some(gates);
    machine.step(&r);
}

fn guest_tick(machine: &mut SaveMachine, now_ms: u64, gates: GuestGates) -> (bool, Vec<String>) {
    let mut r = readings(JumpRole::Guest, now_ms);
    r.gates = Some(gates);
    let step = machine.step(&r);
    (step.request_save, step.logs)
}

fn busy() -> GuestGates {
    GuestGates {
        save_busy: Some(true),
        ..free_and_idle()
    }
}

#[test]
fn the_host_numbers_its_saves_and_tells_each() {
    let mut machine = SaveMachine::new();
    let mut r = readings(JumpRole::Host, 0);
    r.saves_asked_by_the_game = 1;
    assert_eq!(machine.step(&r).tell, vec![1]);
    r.saves_asked_by_the_game = 2;
    let step = machine.step(&r);
    assert_eq!(step.tell, vec![2, 3]);
    assert!(
        step.logs
            .iter()
            .all(|l| l.starts_with("save together: told"))
    );
}

#[test]
fn the_host_tells_nothing_while_it_or_its_partner_is_not_in_play() {
    let mut machine = SaveMachine::new();
    for (local, partner, present, reason) in [
        (false, true, true, "this game is not in play"),
        (true, false, true, "the partner is not in play"),
        (true, true, false, "no campaign partner is linked"),
    ] {
        let mut r = readings(JumpRole::Host, 0);
        r.saves_asked_by_the_game = 1;
        r.local_in_play = local;
        r.partner_in_play = partner;
        r.campaign_partner_present = present;
        let step = machine.step(&r);
        assert!(step.tell.is_empty());
        assert!(step.logs[0].contains(reason), "{:?}", step.logs);
    }
    let mut r = readings(JumpRole::Host, 0);
    r.saves_asked_by_the_game = 1;
    assert_eq!(machine.step(&r).tell, vec![1]);
}

#[test]
fn the_host_tells_at_most_a_few_saves_in_one_tick() {
    let mut machine = SaveMachine::new();
    let mut r = readings(JumpRole::Host, 0);
    r.saves_asked_by_the_game = MOST_HOST_SAVES_TOLD_PER_TICK + 3;
    let step = machine.step(&r);
    assert_eq!(step.tell.len() as u32, MOST_HOST_SAVES_TOLD_PER_TICK);
}

#[test]
fn the_guests_own_saves_and_solo_saves_are_told_to_nobody() {
    for role in [JumpRole::Guest, JumpRole::Solo] {
        let mut machine = SaveMachine::new();
        let mut r = readings(role, 0);
        r.saves_asked_by_the_game = 3;
        let step = machine.step(&r);
        assert!(step.tell.is_empty());
        assert!(!step.request_save);
        assert!(step.logs.is_empty());
    }
}

#[test]
fn a_host_never_acts_on_a_heard_save_and_a_solo_game_ignores_it() {
    for role in [JumpRole::Host, JumpRole::Solo] {
        let mut machine = SaveMachine::new();
        let mut r = readings(role, 0);
        r.host_saves_heard = vec![1];
        r.gates = Some(free_and_idle());
        assert!(!machine.step(&r).request_save);
        assert!(!machine.wants_gates());
    }
}

#[test]
fn the_guest_saves_at_once_when_free_and_idle() {
    let mut machine = SaveMachine::new();
    let mut r = readings(JumpRole::Guest, 1_000);
    r.host_saves_heard = vec![5];
    r.gates = Some(free_and_idle());
    let step = machine.step(&r);
    assert!(step.request_save);
    assert!(step.logs[0].contains("heard host save 5"));
    assert!(step.logs[1].contains("asking the save manager to save, for host save 5"));
}

#[test]
fn the_guest_waits_while_the_player_is_not_free_and_says_why_once_a_second() {
    let mut machine = SaveMachine::new();
    let held = GuestGates {
        player_held_by: Some("a cutscene".to_owned()),
        ..free_and_idle()
    };
    guest_hears(&mut machine, 1, 0, held.clone());
    let (asked, logs) = guest_tick(&mut machine, 100, held.clone());
    assert!(!asked);
    assert!(logs.is_empty());
    let (asked, logs) = guest_tick(&mut machine, SAVE_WAITS_LOG_INTERVAL_MS, held.clone());
    assert!(!asked);
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("the player is not free (a cutscene)"));
    let (_, logs) = guest_tick(&mut machine, SAVE_WAITS_LOG_INTERVAL_MS + 10, held.clone());
    assert!(logs.is_empty());
    let (asked, _) = guest_tick(&mut machine, 2_500, free_and_idle());
    assert!(asked);
}

#[test]
fn the_guest_waits_while_its_save_manager_is_busy() {
    let mut machine = SaveMachine::new();
    guest_hears(&mut machine, 1, 0, busy());
    let (asked, logs) = guest_tick(&mut machine, SAVE_WAITS_LOG_INTERVAL_MS, busy());
    assert!(!asked);
    assert!(logs[0].contains("the save manager is busy"));
    let (asked, _) = guest_tick(&mut machine, 1_500, free_and_idle());
    assert!(asked);
}

#[test]
fn the_guest_waits_while_it_is_not_in_play() {
    let mut machine = SaveMachine::new();
    let mut r = readings(JumpRole::Guest, 0);
    r.host_saves_heard = vec![1];
    r.local_in_play = false;
    r.gates = Some(free_and_idle());
    assert!(!machine.step(&r).request_save);
    r.host_saves_heard.clear();
    r.now_ms = 10;
    r.local_in_play = true;
    assert!(machine.step(&r).request_save);
}

#[test]
fn a_second_host_save_replaces_the_one_that_waits() {
    let mut machine = SaveMachine::new();
    let held = GuestGates {
        player_held_by: Some("x".to_owned()),
        ..free_and_idle()
    };
    guest_hears(&mut machine, 1, 0, held.clone());
    let mut r = readings(JumpRole::Guest, 500);
    r.host_saves_heard = vec![2];
    r.gates = Some(held);
    let step = machine.step(&r);
    assert!(step.logs[0].contains("heard host save 2, it replaces host save 1"));
    let (asked, logs) = guest_tick(&mut machine, 600, free_and_idle());
    assert!(asked);
    assert!(logs[0].contains("host save 2"));
    machine.save_requested(true);
    let (asked, _) = guest_tick(&mut machine, 700, free_and_idle());
    assert!(!asked, "only one save is made for the two");
}

#[test]
fn a_host_save_heard_during_a_write_is_made_after_it() {
    let mut machine = SaveMachine::new();
    guest_hears(&mut machine, 1, 0, free_and_idle());
    machine.save_requested(true);
    guest_hears(&mut machine, 2, 100, busy());
    let (asked, logs) = guest_tick(&mut machine, 200, free_and_idle());
    assert!(logs.iter().any(|l| l.contains("saved for host save 1")));
    assert!(asked);
    assert!(logs.iter().any(|l| l.contains("for host save 2")));
}

#[test]
fn the_guest_gives_up_after_the_wait_limit() {
    let mut machine = SaveMachine::new();
    let held = GuestGates {
        player_held_by: Some("x".to_owned()),
        ..free_and_idle()
    };
    guest_hears(&mut machine, 1, 0, held.clone());
    let (asked, logs) = guest_tick(&mut machine, GUEST_SAVE_WAIT_LIMIT_MS, held.clone());
    assert!(!asked);
    assert!(!logs.iter().any(|l| l.contains("gave up")));
    let (asked, logs) = guest_tick(&mut machine, GUEST_SAVE_WAIT_LIMIT_MS + 1, held);
    assert!(!asked);
    assert!(logs.iter().any(|l| l.contains("gave up on host save 1")));
    assert!(!machine.wants_gates());
}

#[test]
fn the_write_is_done_when_busy_then_idle_and_says_how_long_it_took() {
    let mut machine = SaveMachine::new();
    guest_hears(&mut machine, 1, 1_000, free_and_idle());
    machine.save_requested(false);
    let (_, logs) = guest_tick(&mut machine, 1_100, busy());
    assert!(logs.is_empty());
    let (_, logs) = guest_tick(&mut machine, 1_900, free_and_idle());
    assert_eq!(
        logs,
        vec!["save together: saved for host save 1 after 900 ms"]
    );
    assert!(!machine.wants_gates());
}

#[test]
fn a_write_that_was_busy_right_after_the_call_ends_at_the_first_idle() {
    let mut machine = SaveMachine::new();
    guest_hears(&mut machine, 1, 0, free_and_idle());
    machine.save_requested(true);
    let (_, logs) = guest_tick(&mut machine, 50, free_and_idle());
    assert!(logs[0].contains("saved for host save 1"));
}

#[test]
fn a_failed_write_is_logged_as_a_refusal_and_play_goes_on() {
    let mut machine = SaveMachine::new();
    guest_hears(&mut machine, 1, 0, free_and_idle());
    machine.save_requested(true);
    let failed = GuestGates {
        save_failed: Some(true),
        ..free_and_idle()
    };
    let (_, logs) = guest_tick(&mut machine, 80, failed);
    assert!(logs[0].contains("failed after 80 ms"));
    assert!(!machine.wants_gates());
}

#[test]
fn a_write_the_save_manager_never_took_is_logged_after_a_short_bound() {
    let mut machine = SaveMachine::new();
    guest_hears(&mut machine, 1, 0, free_and_idle());
    machine.save_requested(false);
    let (_, logs) = guest_tick(&mut machine, GUEST_SAVE_NOT_TAKEN_AFTER_MS, free_and_idle());
    assert!(logs.is_empty());
    let (_, logs) = guest_tick(
        &mut machine,
        GUEST_SAVE_NOT_TAKEN_AFTER_MS + 1,
        free_and_idle(),
    );
    assert!(logs[0].contains("never went busy for host save 1"));
}

#[test]
fn a_write_that_runs_past_its_limit_is_given_up_on() {
    let mut machine = SaveMachine::new();
    guest_hears(&mut machine, 1, 0, free_and_idle());
    machine.save_requested(true);
    let (_, logs) = guest_tick(&mut machine, GUEST_SAVE_WRITE_LIMIT_MS, busy());
    assert!(logs.is_empty());
    let (_, logs) = guest_tick(&mut machine, GUEST_SAVE_WRITE_LIMIT_MS + 1, busy());
    assert!(logs[0].contains("still running"));
    assert!(!machine.wants_gates());
}

#[test]
fn the_same_number_heard_twice_makes_one_save_and_a_restarted_host_is_heard_again() {
    let mut machine = SaveMachine::new();
    guest_hears(&mut machine, 1, 0, free_and_idle());
    machine.save_requested(true);
    guest_tick(&mut machine, 10, free_and_idle());
    let mut r = readings(JumpRole::Guest, 20);
    r.host_saves_heard = vec![1];
    r.gates = Some(free_and_idle());
    assert!(!machine.step(&r).request_save, "a repeat is not a new save");
    r.host_saves_heard = vec![3];
    assert!(machine.step(&r).request_save);
    machine.save_requested(true);
    guest_tick(&mut machine, 30, free_and_idle());
    r.host_saves_heard = vec![1];
    assert!(
        machine.step(&r).request_save,
        "a host that restarted counts from 1 again"
    );
}

#[test]
fn a_waiting_save_is_dropped_when_the_link_or_the_role_goes() {
    let mut machine = SaveMachine::new();
    let held = GuestGates {
        player_held_by: Some("x".to_owned()),
        ..free_and_idle()
    };
    guest_hears(&mut machine, 1, 0, held.clone());
    let mut r = readings(JumpRole::Guest, 10);
    r.campaign_partner_present = false;
    r.gates = Some(held);
    let step = machine.step(&r);
    assert!(step.logs[0].contains("host save 1 dropped"));
    assert!(!machine.wants_gates());
    assert!(!step.request_save);
}
