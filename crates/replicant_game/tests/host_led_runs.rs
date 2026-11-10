use replicant_game::script_runner::HostLedRuns;

const HANDLER: &str = "SEID_0001_End";

#[test]
fn nothing_is_host_led_before_an_announcement() {
    let mut runs = HostLedRuns::new();
    runs.holder_popped(HANDLER);
    assert_eq!(runs.running_since_ms(), None);
}

#[test]
fn the_first_run_of_an_announced_name_is_host_led_since_the_announcement() {
    let mut runs = HostLedRuns::new();
    runs.announce(HANDLER, 120);
    runs.holder_popped(HANDLER);
    assert_eq!(runs.running_since_ms(), Some(120));
    assert_eq!(runs.running_name(), Some(HANDLER));
}

#[test]
fn a_later_run_of_the_same_name_that_was_not_announced_is_the_guests_own() {
    let mut runs = HostLedRuns::new();
    runs.announce(HANDLER, 120);
    runs.holder_popped(HANDLER);
    runs.slot_idle();
    runs.holder_popped(HANDLER);
    assert_eq!(runs.running_since_ms(), None);
}

#[test]
fn a_run_of_another_name_is_not_host_led() {
    let mut runs = HostLedRuns::new();
    runs.announce(HANDLER, 120);
    runs.holder_popped("SEID_0002_End");
    assert_eq!(runs.running_since_ms(), None);
    runs.holder_popped(HANDLER);
    assert_eq!(runs.running_since_ms(), Some(120));
}

#[test]
fn leaving_the_slot_ends_the_run() {
    let mut runs = HostLedRuns::new();
    runs.announce(HANDLER, 120);
    runs.holder_popped(HANDLER);
    runs.slot_idle();
    assert_eq!(runs.running_since_ms(), None);
}

#[test]
fn two_announcements_of_one_name_make_two_host_led_runs_in_order() {
    let mut runs = HostLedRuns::new();
    runs.announce(HANDLER, 10);
    runs.announce(HANDLER, 20);
    runs.holder_popped(HANDLER);
    assert_eq!(runs.running_since_ms(), Some(10));
    runs.holder_popped(HANDLER);
    assert_eq!(runs.running_since_ms(), Some(20));
    runs.holder_popped(HANDLER);
    assert_eq!(runs.running_since_ms(), None);
}

#[test]
fn a_phase_change_drops_unused_announcements() {
    let mut runs = HostLedRuns::new();
    runs.enter_phase(Some(7));
    runs.announce(HANDLER, 120);
    runs.enter_phase(Some(8));
    runs.holder_popped(HANDLER);
    assert_eq!(runs.running_since_ms(), None);
}

#[test]
fn a_phase_change_leaves_the_run_in_the_slot_alone() {
    let mut runs = HostLedRuns::new();
    runs.enter_phase(Some(7));
    runs.announce(HANDLER, 120);
    runs.holder_popped(HANDLER);
    runs.enter_phase(Some(8));
    assert_eq!(runs.running_since_ms(), Some(120));
}

#[test]
fn staying_in_the_same_phase_keeps_unused_announcements() {
    let mut runs = HostLedRuns::new();
    runs.enter_phase(Some(7));
    runs.announce(HANDLER, 120);
    runs.enter_phase(Some(7));
    runs.holder_popped(HANDLER);
    assert_eq!(runs.running_since_ms(), Some(120));
}

#[test]
fn the_announcement_keeps_the_arrival_time_it_was_given() {
    let mut runs = HostLedRuns::new();
    runs.announce(HANDLER, 100);
    runs.holder_popped(HANDLER);
    assert_eq!(runs.running_since_ms(), Some(100));
}
