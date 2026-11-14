use crate::downed::DownedPhase;

/// What the overlay needs of the rules each frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DownedView {
    pub local: DownedPhase,
    pub partner: Option<DownedPhase>,
    pub bleed_out_progress: f32,
    pub being_revived_progress: f32,
    pub reviving_progress: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DownedPrompt {
    Nothing,
    Down,
    DownForLong,
    BeingRevived(f32),
    PartnerDown,
    RevivingPartner(f32),
}

impl DownedPrompt {
    /// The fill of the bar under the text, when the prompt has one.
    pub fn bar(self) -> Option<f32> {
        match self {
            Self::BeingRevived(progress) | Self::RevivingPartner(progress) => Some(progress),
            _ => None,
        }
    }
}

pub fn downed_prompt(view: DownedView) -> DownedPrompt {
    match (view.local, view.partner) {
        (DownedPhase::Downed, _) if view.being_revived_progress > 0.0 => {
            DownedPrompt::BeingRevived(view.being_revived_progress.clamp(0.0, 1.0))
        }
        (DownedPhase::Downed, _) if view.bleed_out_progress >= 1.0 => DownedPrompt::DownForLong,
        (DownedPhase::Downed, _) => DownedPrompt::Down,
        (DownedPhase::Up, Some(DownedPhase::Downed)) if view.reviving_progress > 0.0 => {
            DownedPrompt::RevivingPartner(view.reviving_progress.clamp(0.0, 1.0))
        }
        (DownedPhase::Up, Some(DownedPhase::Downed)) => DownedPrompt::PartnerDown,
        _ => DownedPrompt::Nothing,
    }
}

pub fn prompt_text(prompt: DownedPrompt, partner_name: &str) -> Option<String> {
    match prompt {
        DownedPrompt::Nothing => None,
        DownedPrompt::Down => Some(format!("DOWNED: {partner_name} can revive you")),
        DownedPrompt::DownForLong => Some(format!(
            "DOWNED: waiting for {partner_name} (to give up, open the co-op menu)"
        )),
        DownedPrompt::BeingRevived(_) => Some(format!("{partner_name} is reviving you")),
        DownedPrompt::PartnerDown => {
            Some(format!("{partner_name} is DOWNED: stand close to revive"))
        }
        DownedPrompt::RevivingPartner(_) => Some(format!("Reviving {partner_name}")),
    }
}
