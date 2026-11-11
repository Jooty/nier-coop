use crate::downed::DownedState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PartnerBadge {
    Hidden,
    Downed,
    Reviving(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartnerBadgeInputs {
    pub session_active: bool,
    pub local: DownedState,
    pub partner: DownedState,
    pub game_over: bool,
    pub reviving_partner_progress: f32,
    pub being_revived_progress: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LocalDownedLine {
    Nothing,
    BleedingOut(f32),
    WaitingForRevive,
    BeingRevived(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalDownedLineInputs {
    pub local: DownedState,
    pub game_over: bool,
    pub bleed_out_progress: f32,
    pub being_revived_progress: f32,
}

pub fn partner_badge(inputs: PartnerBadgeInputs) -> PartnerBadge {
    if !inputs.session_active || inputs.game_over {
        return PartnerBadge::Hidden;
    }
    match (inputs.local, inputs.partner) {
        (_, DownedState::Downed) if inputs.reviving_partner_progress > 0.0 => {
            PartnerBadge::Reviving(inputs.reviving_partner_progress.clamp(0.0, 1.0))
        }
        (_, DownedState::Downed) => PartnerBadge::Downed,
        (DownedState::Downed, DownedState::Up) if inputs.being_revived_progress > 0.0 => {
            PartnerBadge::Reviving(inputs.being_revived_progress.clamp(0.0, 1.0))
        }
        _ => PartnerBadge::Hidden,
    }
}

pub fn local_downed_line(inputs: LocalDownedLineInputs) -> LocalDownedLine {
    if inputs.local != DownedState::Downed || inputs.game_over {
        return LocalDownedLine::Nothing;
    }
    if inputs.being_revived_progress > 0.0 {
        return LocalDownedLine::BeingRevived(inputs.being_revived_progress.clamp(0.0, 1.0));
    }
    if inputs.bleed_out_progress >= 1.0 {
        return LocalDownedLine::WaitingForRevive;
    }
    LocalDownedLine::BleedingOut(inputs.bleed_out_progress.clamp(0.0, 1.0))
}
