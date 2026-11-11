use replicant_game::kaine::{PartnerHideStep, PartnerKaineHide, partner_kaine_hidden_wanted};
use replicant_proto::PartnerBody;

#[test]
fn she_is_hidden_when_either_player_is_kaine_and_never_otherwise() {
    let kaine = Some(PartnerBody::SeedKaineE);
    let nier = Some(PartnerBody::Nier);
    assert!(partner_kaine_hidden_wanted(kaine, nier));
    assert!(partner_kaine_hidden_wanted(nier, kaine));
    assert!(partner_kaine_hidden_wanted(None, kaine));
    assert!(!partner_kaine_hidden_wanted(
        nier,
        Some(PartnerBody::PrologueCharacter)
    ));
    assert!(!partner_kaine_hidden_wanted(None, None));
}

#[test]
fn an_active_shown_partner_is_hidden_again_every_time_the_game_shows_her() {
    let mut hide = PartnerKaineHide::new();
    assert_eq!(hide.step(true, true, true), PartnerHideStep::Hide);
    assert_eq!(hide.step(true, true, false), PartnerHideStep::Leave);
    assert_eq!(hide.step(true, true, true), PartnerHideStep::Hide);
    assert!(hide.hidden_by_us());
}

#[test]
fn a_partner_the_game_has_not_activated_is_never_touched() {
    let mut hide = PartnerKaineHide::new();
    assert_eq!(hide.step(true, false, false), PartnerHideStep::Leave);
    assert!(!hide.hidden_by_us());
    assert_eq!(hide.step(false, false, false), PartnerHideStep::Leave);
}

#[test]
fn she_is_shown_again_only_if_we_hid_her_and_the_game_still_runs_her() {
    let mut hide = PartnerKaineHide::new();
    hide.step(true, true, true);
    assert_eq!(hide.step(false, true, false), PartnerHideStep::ShowAgain);
    assert!(!hide.hidden_by_us());
    assert_eq!(hide.step(false, true, false), PartnerHideStep::Leave);
}

#[test]
fn a_partner_the_game_stopped_while_hidden_is_left_stopped() {
    let mut hide = PartnerKaineHide::new();
    hide.step(true, true, true);
    assert_eq!(hide.step(false, false, false), PartnerHideStep::Leave);
    assert!(!hide.hidden_by_us());
}
