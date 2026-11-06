use std::time::Duration;

use rc_launcher_core::{PathKind, Quality, RttStats, judge};
use rc_proto::RejectReason;

fn rtt_of(millis: u64) -> Option<RttStats> {
    RttStats::from_samples(&[Duration::from_millis(millis)])
}

#[test]
fn rtt_stats_report_min_average_and_max() {
    let stats = RttStats::from_samples(&[
        Duration::from_millis(10),
        Duration::from_millis(20),
        Duration::from_millis(60),
    ])
    .expect("samples");
    assert_eq!(stats.min, Duration::from_millis(10));
    assert_eq!(stats.avg, Duration::from_millis(30));
    assert_eq!(stats.max, Duration::from_millis(60));
    assert_eq!(RttStats::from_samples(&[]), None);
}

#[test]
fn a_fast_direct_link_is_good() {
    let verdict = judge(true, None, PathKind::Direct, rtt_of(20), 0.0);
    assert_eq!(verdict.quality, Quality::Good);
    assert!(verdict.summary.contains("direct"));
}

#[test]
fn a_relay_is_named_in_plain_words() {
    let verdict = judge(true, None, PathKind::Relay, rtt_of(90), 1.0);
    assert_eq!(verdict.quality, Quality::Fair);
    assert!(verdict.summary.contains("relay"));
}

#[test]
fn heavy_loss_or_delay_is_poor() {
    assert_eq!(
        judge(true, None, PathKind::Direct, rtt_of(30), 12.0).quality,
        Quality::Poor
    );
    assert_eq!(
        judge(true, None, PathKind::Direct, rtt_of(300), 0.0).quality,
        Quality::Poor
    );
}

#[test]
fn an_unreachable_host_is_unusable_and_says_what_to_check() {
    let verdict = judge(false, None, PathKind::Unknown, None, 0.0);
    assert_eq!(verdict.quality, Quality::Unusable);
    assert!(verdict.summary.contains("ticket"));
}

#[test]
fn a_rejection_explains_itself() {
    let verdict = judge(
        true,
        Some(RejectReason::SessionFull),
        PathKind::Unknown,
        None,
        0.0,
    );
    assert_eq!(verdict.quality, Quality::Unusable);
    assert!(verdict.summary.contains("another player"));
}
