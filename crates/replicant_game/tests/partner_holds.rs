use replicant_game::blocks::{
    BLOCK_HELD_REPEAT_INTERVAL_MS, HoldAnnouncement, HoldAnnouncer, HoldChange,
    PARTNER_HOLD_EXPIRY_MS, PartnerHolds,
};

const PHASE: u16 = 4;

fn holds_in_phase() -> PartnerHolds {
    let mut holds = PartnerHolds::default();
    holds.set_local_phase(PHASE);
    holds
}

fn announcement(id: u16, held: bool) -> HoldAnnouncement {
    HoldAnnouncement {
        phase: PHASE,
        id,
        held,
    }
}

#[test]
fn a_partner_hold_is_known_until_the_partner_lets_go() {
    let mut holds = holds_in_phase();
    assert_eq!(
        holds.partner_reports(0, PHASE, 3, true),
        Some(HoldChange::Took(3))
    );
    assert!(holds.holds(3));
    assert_eq!(
        holds.partner_reports(100, PHASE, 3, false),
        Some(HoldChange::LetGo(3))
    );
    assert!(!holds.holds(3));
}

#[test]
fn repeats_keep_a_hold_past_the_expiry_time() {
    let mut holds = holds_in_phase();
    holds.partner_reports(0, PHASE, 3, true);
    let mut now = 0;
    while now < PARTNER_HOLD_EXPIRY_MS * 3 {
        now += BLOCK_HELD_REPEAT_INTERVAL_MS;
        assert_eq!(holds.partner_reports(now, PHASE, 3, true), None);
        assert!(holds.expire(now).is_empty());
    }
    assert!(holds.holds(3));
}

#[test]
fn a_hold_without_repeats_expires() {
    let mut holds = holds_in_phase();
    holds.partner_reports(0, PHASE, 3, true);
    assert!(holds.expire(PARTNER_HOLD_EXPIRY_MS - 1).is_empty());
    assert_eq!(
        holds.expire(PARTNER_HOLD_EXPIRY_MS),
        vec![HoldChange::Unconfirmed(3)]
    );
    assert!(!holds.holds(3));
}

#[test]
fn a_local_phase_change_drops_every_hold() {
    let mut holds = holds_in_phase();
    holds.partner_reports(0, PHASE, 3, true);
    assert!(holds.set_local_phase(PHASE).is_empty());
    assert_eq!(
        holds.set_local_phase(PHASE + 1),
        vec![HoldChange::Unconfirmed(3)]
    );
    assert!(!holds.holds(3));
}

#[test]
fn a_report_from_another_phase_is_ignored() {
    let mut holds = holds_in_phase();
    assert_eq!(holds.partner_reports(0, PHASE + 1, 3, true), None);
    assert!(!holds.holds(3));
}

#[test]
fn the_partner_going_away_clears_every_hold() {
    let mut holds = holds_in_phase();
    holds.partner_reports(0, PHASE, 3, true);
    holds.partner_reports(0, PHASE, 5, true);
    assert_eq!(
        holds.clear(),
        vec![HoldChange::Unconfirmed(3), HoldChange::Unconfirmed(5)]
    );
    assert!(holds.held_ids().is_empty());
}

#[test]
fn two_blocks_are_held_and_released_independently() {
    let mut holds = holds_in_phase();
    holds.partner_reports(0, PHASE, 3, true);
    holds.partner_reports(0, PHASE, 5, true);
    holds.partner_reports(10, PHASE, 3, false);
    assert!(!holds.holds(3));
    assert!(holds.holds(5));
}

#[test]
fn the_announcer_sends_start_repeats_and_release() {
    let mut announcer = HoldAnnouncer::default();
    assert!(announcer.observe(0, PHASE, None).is_empty());
    assert_eq!(
        announcer.observe(10, PHASE, Some(2)),
        vec![announcement(2, true)]
    );
    assert!(announcer.observe(20, PHASE, Some(2)).is_empty());
    assert_eq!(
        announcer.observe(10 + BLOCK_HELD_REPEAT_INTERVAL_MS, PHASE, Some(2)),
        vec![announcement(2, true)]
    );
    assert_eq!(
        announcer.observe(700, PHASE, None),
        vec![announcement(2, false)]
    );
    assert!(announcer.observe(800, PHASE, None).is_empty());
}

#[test]
fn the_announcer_releases_when_the_local_phase_changes() {
    let mut announcer = HoldAnnouncer::default();
    announcer.observe(0, PHASE, Some(2));
    assert_eq!(
        announcer.observe(10, PHASE + 1, Some(2)),
        vec![
            announcement(2, false),
            HoldAnnouncement {
                phase: PHASE + 1,
                id: 2,
                held: true
            }
        ]
    );
}

#[test]
fn the_announcer_switches_blocks_with_a_release_and_a_start() {
    let mut announcer = HoldAnnouncer::default();
    announcer.observe(0, PHASE, Some(2));
    assert_eq!(
        announcer.observe(10, PHASE, Some(7)),
        vec![announcement(2, false), announcement(7, true)]
    );
}
