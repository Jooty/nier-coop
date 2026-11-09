use rc_game::downed::{
    DownedState, LocalDownedLine, LocalDownedLineInputs, PartnerBadge, PartnerBadgeInputs,
    local_downed_line, partner_badge,
};

fn badge_inputs(local: DownedState, partner: DownedState) -> PartnerBadgeInputs {
    PartnerBadgeInputs {
        session_active: true,
        local,
        partner,
        game_over: false,
        reviving_partner_progress: 0.0,
        being_revived_progress: 0.0,
    }
}

fn line_inputs(local: DownedState) -> LocalDownedLineInputs {
    LocalDownedLineInputs {
        local,
        game_over: false,
        bleed_out_progress: 0.3,
        being_revived_progress: 0.0,
    }
}

#[test]
fn nobody_down_shows_no_badge() {
    let inputs = badge_inputs(DownedState::Up, DownedState::Up);
    assert_eq!(partner_badge(inputs), PartnerBadge::Hidden);
}

#[test]
fn a_downed_partner_shows_the_downed_badge() {
    let inputs = badge_inputs(DownedState::Up, DownedState::Downed);
    assert_eq!(partner_badge(inputs), PartnerBadge::Downed);
}

#[test]
fn holding_a_revive_on_the_downed_partner_shows_the_progress() {
    let mut inputs = badge_inputs(DownedState::Up, DownedState::Downed);
    inputs.reviving_partner_progress = 0.4;
    assert_eq!(partner_badge(inputs), PartnerBadge::Reviving(0.4));
}

#[test]
fn being_revived_shows_the_progress_above_the_reviver() {
    let mut inputs = badge_inputs(DownedState::Downed, DownedState::Up);
    inputs.being_revived_progress = 0.75;
    assert_eq!(partner_badge(inputs), PartnerBadge::Reviving(0.75));
}

#[test]
fn a_downed_player_with_nobody_reviving_shows_nothing_above_the_up_partner() {
    let inputs = badge_inputs(DownedState::Downed, DownedState::Up);
    assert_eq!(partner_badge(inputs), PartnerBadge::Hidden);
}

#[test]
fn an_up_player_never_shows_a_bar_over_an_up_partner() {
    let mut inputs = badge_inputs(DownedState::Up, DownedState::Up);
    inputs.being_revived_progress = 0.5;
    inputs.reviving_partner_progress = 0.5;
    assert_eq!(partner_badge(inputs), PartnerBadge::Hidden);
}

#[test]
fn both_down_ends_the_badges() {
    let mut inputs = badge_inputs(DownedState::Downed, DownedState::Downed);
    inputs.game_over = true;
    assert_eq!(partner_badge(inputs), PartnerBadge::Hidden);
}

#[test]
fn no_session_shows_no_badge() {
    let mut inputs = badge_inputs(DownedState::Up, DownedState::Downed);
    inputs.session_active = false;
    assert_eq!(partner_badge(inputs), PartnerBadge::Hidden);
}

#[test]
fn the_panel_shows_the_bleed_bar_while_down() {
    let inputs = line_inputs(DownedState::Downed);
    assert_eq!(local_downed_line(inputs), LocalDownedLine::BleedingOut(0.3));
}

#[test]
fn a_capped_bleed_bar_says_waiting_for_revive() {
    let mut inputs = line_inputs(DownedState::Downed);
    inputs.bleed_out_progress = 1.0;
    assert_eq!(local_downed_line(inputs), LocalDownedLine::WaitingForRevive);
}

#[test]
fn being_revived_replaces_the_bleed_line_with_the_revive_progress() {
    let mut inputs = line_inputs(DownedState::Downed);
    inputs.bleed_out_progress = 1.0;
    inputs.being_revived_progress = 0.5;
    assert_eq!(
        local_downed_line(inputs),
        LocalDownedLine::BeingRevived(0.5)
    );
}

#[test]
fn the_panel_line_is_empty_when_up_or_game_over() {
    assert_eq!(
        local_downed_line(line_inputs(DownedState::Up)),
        LocalDownedLine::Nothing
    );
    let mut over = line_inputs(DownedState::Downed);
    over.game_over = true;
    assert_eq!(local_downed_line(over), LocalDownedLine::Nothing);
}
