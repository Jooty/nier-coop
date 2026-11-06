use rc_game::readiness::{
    JOINING_IGNORED_AFTER_MS, LocalGameReadiness, PartnerWatch, PlayChecks,
    SILENT_PARTNER_FALLBACK_MS, classify, joining_label,
};
use rc_proto::GameReadiness;

const ALL_READY: PlayChecks = PlayChecks {
    phase_known: true,
    player_actor_spawned: true,
    vitals_readable: true,
};

#[test]
fn a_game_with_everything_readable_is_in_play() {
    assert_eq!(classify(ALL_READY), GameReadiness::InPlay);
}

#[test]
fn a_missing_phase_actor_or_vitals_means_loading() {
    for checks in [
        PlayChecks {
            phase_known: false,
            ..ALL_READY
        },
        PlayChecks {
            player_actor_spawned: false,
            ..ALL_READY
        },
        PlayChecks {
            vitals_readable: false,
            ..ALL_READY
        },
        PlayChecks::default(),
    ] {
        assert_eq!(classify(checks), GameReadiness::Loading);
    }
}

#[test]
fn an_unlinked_game_is_not_linked_and_ignores_reports() {
    let mut local = LocalGameReadiness::new();
    assert_eq!(local.current(), GameReadiness::NotLinked);
    local.game_reported(GameReadiness::InPlay);
    assert_eq!(local.current(), GameReadiness::NotLinked);
}

#[test]
fn a_freshly_linked_game_is_loading_until_it_reports_in_play() {
    let mut local = LocalGameReadiness::new();
    local.game_linked();
    assert_eq!(local.current(), GameReadiness::Loading);
    local.game_reported(GameReadiness::InPlay);
    assert_eq!(local.current(), GameReadiness::InPlay);
    local.game_reported(GameReadiness::Loading);
    assert_eq!(local.current(), GameReadiness::Loading);
}

#[test]
fn a_game_cannot_report_itself_as_not_linked() {
    let mut local = LocalGameReadiness::new();
    local.game_linked();
    local.game_reported(GameReadiness::NotLinked);
    assert_eq!(local.current(), GameReadiness::Loading);
}

#[test]
fn a_game_that_closes_goes_back_to_not_linked_and_loading_on_return() {
    let mut local = LocalGameReadiness::new();
    local.game_linked();
    local.game_reported(GameReadiness::InPlay);
    local.game_unlinked();
    assert_eq!(local.current(), GameReadiness::NotLinked);
    local.game_linked();
    assert_eq!(local.current(), GameReadiness::Loading);
}

#[test]
fn a_partner_who_just_connected_is_joining_not_in_play() {
    let mut partner = PartnerWatch::new();
    assert!(!partner.is_joining());
    partner.partner_connected(1_000);
    assert!(partner.is_joining());
    assert!(!partner.is_in_play());
    partner.partner_reported(GameReadiness::Loading);
    assert!(partner.is_joining());
}

#[test]
fn a_partner_reporting_in_play_is_no_longer_joining() {
    let mut partner = PartnerWatch::new();
    partner.partner_connected(0);
    partner.partner_reported(GameReadiness::InPlay);
    assert!(partner.is_in_play());
    assert!(!partner.is_joining());
}

#[test]
fn a_partner_whose_game_closes_goes_back_to_joining() {
    let mut partner = PartnerWatch::new();
    partner.partner_connected(0);
    partner.partner_reported(GameReadiness::InPlay);
    partner.partner_reported(GameReadiness::NotLinked);
    assert!(!partner.is_in_play());
    assert!(partner.is_joining());
}

#[test]
fn a_report_without_a_connected_partner_is_ignored() {
    let mut partner = PartnerWatch::new();
    partner.partner_reported(GameReadiness::InPlay);
    assert!(!partner.is_in_play());
    partner.partner_connected(0);
    partner.partner_gone();
    assert_eq!(partner.readiness(), GameReadiness::NotLinked);
    assert!(!partner.is_joining());
}

#[test]
fn a_joining_partner_counts_for_the_zone_rule_until_the_timeout() {
    let mut partner = PartnerWatch::new();
    assert!(!partner.counts_for_zone_rule(0));
    partner.partner_connected(5_000);
    assert!(partner.counts_for_zone_rule(5_000 + JOINING_IGNORED_AFTER_MS - 1));
    assert!(!partner.counts_for_zone_rule(5_000 + JOINING_IGNORED_AFTER_MS));
}

#[test]
fn an_in_play_partner_always_counts_for_the_zone_rule() {
    let mut partner = PartnerWatch::new();
    partner.partner_connected(0);
    partner.partner_reported(GameReadiness::InPlay);
    assert!(partner.counts_for_zone_rule(JOINING_IGNORED_AFTER_MS * 10));
}

#[test]
fn the_joining_label_names_what_the_partner_is_doing() {
    assert!(joining_label(GameReadiness::NotLinked).contains("not open"));
    assert!(joining_label(GameReadiness::Loading).contains("loading"));
}

#[test]
fn a_partner_that_never_reports_is_assumed_in_play_after_the_fallback() {
    let mut partner = PartnerWatch::new();
    partner.partner_connected(1_000);
    assert!(!partner.assume_in_play_if_silent(1_000 + SILENT_PARTNER_FALLBACK_MS - 1));
    assert!(partner.assume_in_play_if_silent(1_000 + SILENT_PARTNER_FALLBACK_MS));
    assert!(partner.is_in_play());
}

#[test]
fn a_partner_that_reported_loading_is_never_assumed_in_play() {
    let mut partner = PartnerWatch::new();
    partner.partner_connected(0);
    partner.partner_reported(GameReadiness::Loading);
    assert!(!partner.assume_in_play_if_silent(SILENT_PARTNER_FALLBACK_MS * 10));
    assert!(partner.is_joining());
}

#[test]
fn a_state_from_the_partner_proves_it_is_in_play() {
    let mut partner = PartnerWatch::new();
    assert!(!partner.partner_sent_state());
    partner.partner_connected(0);
    partner.partner_reported(GameReadiness::Loading);
    assert!(partner.partner_sent_state());
    assert!(partner.is_in_play());
    assert!(!partner.partner_sent_state());
}

#[test]
fn a_partner_whose_game_closes_is_flagged_closed_once() {
    let mut partner = PartnerWatch::new();
    partner.partner_connected(0);
    assert!(!partner.partner_reported(GameReadiness::NotLinked));
    assert!(!partner.is_game_closed());
    partner.partner_reported(GameReadiness::InPlay);
    assert!(partner.partner_reported(GameReadiness::NotLinked));
    assert!(partner.is_game_closed());
    assert!(!partner.partner_reported(GameReadiness::NotLinked));
    assert!(partner.is_game_closed());
}

#[test]
fn a_closed_partner_does_not_hold_up_zone_changes_and_reopening_clears_it() {
    let mut partner = PartnerWatch::new();
    partner.partner_connected(0);
    partner.partner_reported(GameReadiness::InPlay);
    partner.partner_reported(GameReadiness::NotLinked);
    assert!(!partner.counts_for_zone_rule(1_000));
    partner.partner_reported(GameReadiness::Loading);
    assert!(!partner.is_game_closed());
    assert!(partner.counts_for_zone_rule(1_000));
}

#[test]
fn a_closed_partner_sending_its_position_again_is_no_longer_closed() {
    let mut partner = PartnerWatch::new();
    partner.partner_connected(0);
    partner.partner_reported(GameReadiness::InPlay);
    partner.partner_reported(GameReadiness::NotLinked);
    assert!(partner.partner_sent_state());
    assert!(!partner.is_game_closed());
}
