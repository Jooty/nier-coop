use replicant_game::carts::{
    CART_FRAME_SET_GAP_FRAMES, CART_HOST_RUN_SILENT_MS, CART_HOST_RUN_STILL_MS,
    CART_RUN_END_FRAME_WHEN_UNREAD, CART_STEERED_RATE_MAX, CART_STEERED_RATE_MIN, CartLeftAlone,
    CartSteer, HostCartRun, HostCartRunTrack, steer_cart,
};

const END: f32 = CART_RUN_END_FRAME_WHEN_UNREAD;
const NOW: u64 = 50_000;

fn host_at(frame: f32, rate: f32) -> HostCartRun {
    HostCartRun {
        seq: 7,
        frame,
        rate,
        received_ms: NOW,
        moved_ms: NOW,
    }
}

#[test]
fn a_small_gap_is_closed_by_rate() {
    let host = host_at(1_000.0, 1.0);
    let CartSteer::Rate { rate, gap } = steer_cart(Some(&host), 994.0, END, NOW) else {
        panic!("a six-frame gap is steered by rate");
    };
    assert_eq!(gap, 6.0);
    assert!(
        rate > 1.0,
        "behind the host: faster than the host, got {rate}"
    );
    let CartSteer::Rate { rate, gap } = steer_cart(Some(&host), 1_006.0, END, NOW) else {
        panic!("a six-frame lead is steered by rate");
    };
    assert_eq!(gap, -6.0);
    assert!(
        rate < 1.0,
        "ahead of the host: slower than the host, got {rate}"
    );
    let CartSteer::Rate { rate, .. } = steer_cart(Some(&host), 1_000.0, END, NOW) else {
        panic!("no gap is still steered");
    };
    assert_eq!(rate, 1.0, "no gap: the host's own rate");
}

#[test]
fn the_host_frame_is_carried_forward_from_its_arrival() {
    let host = host_at(1_000.0, 1.5);
    let CartSteer::Rate { gap, .. } = steer_cart(Some(&host), 1_009.0, END, NOW + 100) else {
        panic!("steered by rate");
    };
    assert!(
        gap.abs() < 0.01,
        "100 ms at rate 1.5 is 9 frames, got a gap of {gap}"
    );
}

#[test]
fn a_large_gap_sets_the_frame() {
    let host = host_at(2_000.0, 1.7);
    let behind = steer_cart(
        Some(&host),
        2_000.0 - CART_FRAME_SET_GAP_FRAMES - 5.0,
        END,
        NOW,
    );
    let CartSteer::SetFrame { frame, rate, .. } = behind else {
        panic!("far behind: the frame is set, got {behind:?}");
    };
    assert_eq!(frame, 2_000.0);
    assert_eq!(rate, 1.7);
    let ahead = steer_cart(Some(&host), 2_100.0, END, NOW);
    assert!(
        matches!(ahead, CartSteer::SetFrame { frame, .. } if frame == 2_000.0),
        "far ahead: set back to the host, got {ahead:?}"
    );
}

#[test]
fn the_frame_is_never_set_to_the_end_or_past_it() {
    for (host_frame, own_frame) in [(END + 200.0, END - 300.0), (END, 100.0), (END - 1.0, 0.0)] {
        let host = host_at(host_frame, 1.0);
        match steer_cart(Some(&host), own_frame, END, NOW) {
            CartSteer::SetFrame { frame, .. } => assert!(frame < END, "set to {frame}"),
            other => panic!("a gap this large sets the frame, got {other:?}"),
        }
    }
    let host = host_at(END + 200.0, 1.0);
    assert!(
        matches!(
            steer_cart(Some(&host), END - 4.0, END, NOW),
            CartSteer::Rate { rate, .. } if rate > 0.0
        ),
        "a cart already at the last settable frame runs out by rate"
    );
}

#[test]
fn the_rate_is_never_zero_or_negative() {
    for (host_rate, own_frame) in [
        (0.0, 1_020.0),
        (-3.0, 1_029.0),
        (0.3, 1_029.0),
        (5.0, 971.0),
    ] {
        let host = host_at(1_000.0, host_rate);
        let rate = match steer_cart(Some(&host), own_frame, END, NOW) {
            CartSteer::Rate { rate, .. } | CartSteer::SetFrame { rate, .. } => rate,
            CartSteer::LeaveAlone(why) => panic!("steered, not left alone ({why:?})"),
        };
        assert!(
            (CART_STEERED_RATE_MIN..=CART_STEERED_RATE_MAX).contains(&rate) && rate > 0.0,
            "host rate {host_rate}, own frame {own_frame}: rate {rate}"
        );
    }
}

#[test]
fn a_cart_without_a_host_ride_is_left_alone() {
    assert_eq!(
        steer_cart(None, 500.0, END, NOW),
        CartSteer::LeaveAlone(CartLeftAlone::NoHostRide)
    );
    let host = host_at(500.0, 1.0);
    assert_eq!(
        steer_cart(Some(&host), 500.0, END, NOW + CART_HOST_RUN_SILENT_MS + 1),
        CartSteer::LeaveAlone(CartLeftAlone::HostSilent)
    );
    assert_eq!(
        steer_cart(Some(&host), END + 0.5, END, NOW),
        CartSteer::LeaveAlone(CartLeftAlone::AtItsEnd)
    );
}

#[test]
fn a_paused_host_never_holds_the_guest_cart_back() {
    let mut track = HostCartRunTrack::new();
    assert!(track.receive(1, 800.0, 1.0, NOW));
    assert!(track.receive(2, 800.0, 1.0, NOW + 200));
    assert!(track.receive(3, 800.0, 1.0, NOW + CART_HOST_RUN_STILL_MS + 50));
    let host = track.latest().expect("a run");
    assert_eq!(
        steer_cart(Some(&host), 850.0, END, NOW + CART_HOST_RUN_STILL_MS + 60),
        CartSteer::LeaveAlone(CartLeftAlone::HostStill)
    );
    assert!(track.receive(4, 806.0, 1.0, NOW + CART_HOST_RUN_STILL_MS + 100));
    assert!(matches!(
        steer_cart(
            track.latest().as_ref(),
            850.0,
            END,
            NOW + CART_HOST_RUN_STILL_MS + 100
        ),
        CartSteer::SetFrame { .. }
    ));
}

#[test]
fn an_older_progress_message_never_replaces_a_newer_one() {
    let mut track = HostCartRunTrack::new();
    assert!(track.receive(5, 900.0, 1.0, NOW));
    assert!(!track.receive(4, 850.0, 1.0, NOW + 10));
    assert!(!track.receive(5, 850.0, 1.0, NOW + 10));
    assert_eq!(track.latest().map(|run| run.frame), Some(900.0));
    track.clear();
    assert_eq!(track.latest(), None);
}
