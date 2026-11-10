use rc_game::fight_sync::{
    GuestRadioWaits, HostRadioWaits, RADIO_WAIT_BEHIND_STREAM_GRACE_MS, RadioState,
    RadioWaitContext, RadioWaitMark, RadioWaitPoll, RadioWaitRelease, RadioWaitRun,
    STREAM_STALL_LOGGED_EVERY_MS, STREAM_STALL_LOGGED_FROM_MS, StreamStall, StreamStallWatch,
    format_radio_line_under_menu, format_radio_wait_release, format_stream_stall,
};

const SCENE_HANDLER: &str = "EID_0200_a0040g_Talk";
const ENTRY_42: RadioWaitRun = RadioWaitRun::Entry { epoch: 1, seq: 42 };
const ENTRY_43: RadioWaitRun = RadioWaitRun::Entry { epoch: 1, seq: 43 };
const NOTHING_KNOWN: RadioWaitContext = RadioWaitContext {
    entry_behind_ms: None,
    host_ended_entry: false,
    host_in_scene: false,
    here_in_scene: false,
};

fn starts(run: RadioWaitRun, now_ms: u64) -> RadioWaitPoll {
    RadioWaitPoll {
        run,
        starts: true,
        still_waiting: true,
        now_ms,
    }
}

fn waits(run: RadioWaitRun, now_ms: u64) -> RadioWaitPoll {
    RadioWaitPoll {
        starts: false,
        ..starts(run, now_ms)
    }
}

fn ends(run: RadioWaitRun, now_ms: u64) -> RadioWaitPoll {
    RadioWaitPoll {
        still_waiting: false,
        ..waits(run, now_ms)
    }
}

fn mark(seq: u64, wait_index: u32, host_waited_ms: u64) -> RadioWaitMark {
    RadioWaitMark {
        epoch: 1,
        seq,
        wait_index,
        host_waited_ms,
    }
}

#[test]
fn the_host_marks_each_wait_of_a_streamed_entry_by_its_index_and_length() {
    let mut host = HostRadioWaits::new();
    assert_eq!(host.polled(starts(ENTRY_42, 1_000)), None);
    assert_eq!(host.polled(waits(ENTRY_42, 2_000)), None);
    assert_eq!(host.polled(ends(ENTRY_42, 9_000)), Some(mark(42, 0, 8_000)));
    // A line the game could not start ends in the call that began it.
    let failed_start = RadioWaitPoll {
        still_waiting: false,
        ..starts(ENTRY_42, 9_500)
    };
    assert_eq!(host.polled(failed_start), Some(mark(42, 1, 0)));
    assert_eq!(host.polled(starts(ENTRY_43, 10_000)), None);
    assert_eq!(
        host.polled(ends(ENTRY_43, 10_400)),
        Some(mark(43, 0, 400)),
        "each entry counts its waits from 0"
    );
}

#[test]
fn the_host_sends_nothing_for_a_wait_outside_a_streamed_entry() {
    let mut host = HostRadioWaits::new();
    assert_eq!(host.polled(starts(RadioWaitRun::Other, 0)), None);
    assert_eq!(host.polled(ends(RadioWaitRun::Other, 3_000)), None);
}

#[test]
fn a_parked_radio_wait_is_released_by_its_mark_once_it_waited_as_long_as_the_hosts() {
    let mut guest = GuestRadioWaits::new();
    guest.receive_mark(mark(42, 0, 8_000));
    assert_eq!(guest.polled(starts(ENTRY_42, 20_000), NOTHING_KNOWN), None);
    assert_eq!(
        guest.polled(waits(ENTRY_42, 27_999), NOTHING_KNOWN),
        None,
        "the line plays as long here as on the host"
    );
    assert_eq!(
        guest.polled(waits(ENTRY_42, 28_000), NOTHING_KNOWN),
        Some(RadioWaitRelease::Mark)
    );
}

#[test]
fn a_mark_that_comes_while_the_wait_is_parked_releases_it_at_once_when_it_waited_longer() {
    let mut guest = GuestRadioWaits::new();
    assert_eq!(guest.polled(starts(ENTRY_42, 0), NOTHING_KNOWN), None);
    assert_eq!(guest.polled(waits(ENTRY_42, 30_000), NOTHING_KNOWN), None);
    guest.receive_mark(mark(42, 0, 8_000));
    assert_eq!(
        guest.polled(waits(ENTRY_42, 30_016), NOTHING_KNOWN),
        Some(RadioWaitRelease::Mark)
    );
}

#[test]
fn a_mark_for_a_wait_that_already_ended_here_is_ignored() {
    let mut guest = GuestRadioWaits::new();
    assert_eq!(guest.polled(starts(ENTRY_42, 0), NOTHING_KNOWN), None);
    assert_eq!(guest.polled(ends(ENTRY_42, 2_000), NOTHING_KNOWN), None);
    guest.receive_mark(mark(42, 0, 0));
    // The handler's next wait is its second one: the first one's mark never ends it.
    assert_eq!(guest.polled(starts(ENTRY_42, 2_100), NOTHING_KNOWN), None);
    assert_eq!(guest.polled(waits(ENTRY_42, 60_000), NOTHING_KNOWN), None);
}

#[test]
fn a_mark_of_an_entry_this_game_already_passed_is_dropped() {
    let mut guest = GuestRadioWaits::new();
    assert_eq!(guest.polled(starts(ENTRY_43, 0), NOTHING_KNOWN), None);
    guest.receive_mark(mark(42, 0, 0));
    assert_eq!(guest.marks_kept(), 0);
    guest.receive_mark(mark(44, 0, 0));
    assert_eq!(
        guest.marks_kept(),
        1,
        "a mark of a later entry waits for it"
    );
}

#[test]
fn catch_up_releases_a_wait_after_the_grace_and_not_before() {
    let grace = RADIO_WAIT_BEHIND_STREAM_GRACE_MS;
    let behind = |ms: u64| RadioWaitContext {
        entry_behind_ms: Some(ms),
        ..NOTHING_KNOWN
    };
    for run in [RadioWaitRun::Other, ENTRY_42] {
        let mut guest = GuestRadioWaits::new();
        assert_eq!(guest.polled(starts(run, 0), behind(0)), None);
        assert_eq!(guest.polled(waits(run, grace), behind(grace)), None);
        assert_eq!(
            guest.polled(waits(run, grace + 1), behind(grace + 1)),
            Some(RadioWaitRelease::HostMovedOn),
            "{run:?}"
        );
    }
}

#[test]
fn catch_up_never_cuts_a_wait_short_of_the_hosts_copy_of_it() {
    let mut guest = GuestRadioWaits::new();
    guest.receive_mark(mark(42, 0, 8_000));
    let behind = RadioWaitContext {
        entry_behind_ms: Some(60_000),
        ..NOTHING_KNOWN
    };
    assert_eq!(guest.polled(starts(ENTRY_42, 0), behind), None);
    assert_eq!(guest.polled(waits(ENTRY_42, 7_999), behind), None);
    assert_eq!(
        guest.polled(waits(ENTRY_42, 8_000), behind),
        Some(RadioWaitRelease::Mark)
    );
}

#[test]
fn the_hosts_end_of_the_entry_releases_a_wait_the_host_never_had() {
    let mut guest = GuestRadioWaits::new();
    let host_ended = RadioWaitContext {
        host_ended_entry: true,
        ..NOTHING_KNOWN
    };
    assert_eq!(
        guest.polled(starts(ENTRY_42, 0), host_ended),
        Some(RadioWaitRelease::HostMovedOn)
    );
}

#[test]
fn the_host_in_a_scene_releases_a_parked_wait_unless_this_game_is_in_one_too() {
    let host_in_scene = RadioWaitContext {
        host_in_scene: true,
        ..NOTHING_KNOWN
    };
    let both_in_scene = RadioWaitContext {
        here_in_scene: true,
        ..host_in_scene
    };
    let mut guest = GuestRadioWaits::new();
    assert_eq!(
        guest.polled(starts(RadioWaitRun::Other, 0), both_in_scene),
        None
    );
    assert_eq!(
        guest.polled(waits(RadioWaitRun::Other, 16), host_in_scene),
        Some(RadioWaitRelease::HostInScene)
    );
}

#[test]
fn a_wait_with_no_entry_behind_no_mark_and_no_host_scene_is_never_released() {
    let mut guest = GuestRadioWaits::new();
    for run in [RadioWaitRun::Other, ENTRY_42] {
        assert_eq!(guest.polled(starts(run, 0), NOTHING_KNOWN), None);
        for now_ms in (0..600_000).step_by(16_667) {
            assert_eq!(guest.polled(waits(run, now_ms), NOTHING_KNOWN), None);
        }
    }
}

#[test]
fn the_release_line_names_the_handler_and_the_reason() {
    assert_eq!(
        format_radio_wait_release(SCENE_HANDLER, RadioWaitRelease::Mark),
        "radio wait: ended your wait in EID_0200_a0040g_Talk (mark)"
    );
    assert_eq!(
        format_radio_wait_release(SCENE_HANDLER, RadioWaitRelease::HostMovedOn),
        "radio wait: ended your wait in EID_0200_a0040g_Talk (the host moved on)"
    );
    assert_eq!(
        format_radio_wait_release(SCENE_HANDLER, RadioWaitRelease::HostInScene),
        "radio wait: ended your wait in EID_0200_a0040g_Talk (the host is in a scene)"
    );
}

#[test]
fn the_stall_line_fires_past_five_seconds_then_once_per_five_seconds() {
    let from = STREAM_STALL_LOGGED_FROM_MS;
    let every = STREAM_STALL_LOGGED_EVERY_MS;
    let mut watch = StreamStallWatch::new();
    watch.observe(Some(43), 1_000);
    assert_eq!(watch.stall_due(1_000 + from), None);
    assert_eq!(watch.stall_due(1_001 + from), Some((43, from + 1)));
    assert_eq!(watch.stall_due(1_001 + from + every), None);
    assert_eq!(
        watch.stall_due(1_002 + from + every),
        Some((43, from + every + 2))
    );
    watch.observe(Some(43), 30_000);
    assert_eq!(
        watch.waiting_ms(30_000),
        Some(29_000),
        "the same entry keeps its wait"
    );
    watch.observe(Some(44), 30_000);
    assert_eq!(
        watch.stall_due(30_000 + from),
        None,
        "a new entry starts its own wait"
    );
    watch.observe(None, 40_000);
    assert_eq!(watch.stall_due(90_000), None);
}

#[test]
fn the_stall_line_names_the_entry_its_holder_the_native_it_waits_in_and_the_radio() {
    let radio = RadioState {
        abort_flag: false,
        event_scene: 0,
        active: 1,
        busy: 1,
        line_state: 4,
        line_time_s: 2.5,
        line_flag: 0,
    };
    let stall = StreamStall {
        seq: 43,
        name: "EID_0200_a0040g_02_Start",
        waited_ms: 5_001,
        holder: Some((SCENE_HANDLER, 11_000)),
        last_yielding_native: Some(("_StartRadio", 0)),
        radio: Some(radio),
    };
    assert_eq!(
        format_stream_stall(&stall),
        "fight stream: seq 43 EID_0200_a0040g_02_Start waits behind EID_0200_a0040g_Talk (in the slot 11000 ms, last yielding native _StartRadio 0 ms ago, radio active 1, busy 1, line state 4, line time 2.50 s, line flag 0, abort flag false, event scene 0), waited 5001 ms"
    );
    assert_eq!(
        format_radio_line_under_menu(SCENE_HANDLER, &radio),
        "radio: a line started with a menu open in EID_0200_a0040g_Talk, radio active 1, busy 1, line state 4, line time 2.50 s, line flag 0, abort flag false, event scene 0"
    );
}
