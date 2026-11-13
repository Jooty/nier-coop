use std::time::Duration;

use nc_launcher::app::precheck_failure;
use nc_launcher::net::{ConnectionReport, PathKind, RttStats};

fn report(reachable: bool, with_rtt: bool, verdict: &str) -> ConnectionReport {
    let sample = Duration::from_millis(20);
    ConnectionReport {
        reachable,
        path: PathKind::Direct,
        rtt: with_rtt.then_some(RttStats {
            min: sample,
            avg: sample,
            max: sample,
        }),
        loss_percent: 0.0,
        rejected: None,
        verdict: verdict.to_owned(),
    }
}

#[test]
fn working_connection_lets_join_continue() {
    assert_eq!(precheck_failure(&report(true, true, "good")), None);
}

#[test]
fn unreachable_peer_stops_join_with_the_verdict() {
    let text = precheck_failure(&report(false, false, "Could not reach the host."));
    assert_eq!(text.as_deref(), Some("Could not reach the host."));
}

#[test]
fn rejection_stops_join_with_the_verdict() {
    let text = precheck_failure(&report(
        true,
        false,
        "The host runs a different launcher version.",
    ));
    assert_eq!(
        text.as_deref(),
        Some("The host runs a different launcher version.")
    );
}
