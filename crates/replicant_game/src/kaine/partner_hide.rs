use replicant_proto::PartnerBody;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartnerHideStep {
    Leave,
    Hide,
    ShowAgain,
}

/// Hides the AI partner Kaine by her drawn byte only, so her AI and every script that moves or
/// waits on her keep running; her execute byte stays the game's word on whether she is active.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PartnerKaineHide {
    hidden_by_us: bool,
}

impl PartnerKaineHide {
    pub const fn new() -> Self {
        Self {
            hidden_by_us: false,
        }
    }

    pub fn hidden_by_us(&self) -> bool {
        self.hidden_by_us
    }

    pub fn step(&mut self, wanted: bool, executes: bool, drawn: bool) -> PartnerHideStep {
        if wanted {
            if executes && drawn {
                self.hidden_by_us = true;
                return PartnerHideStep::Hide;
            }
            return PartnerHideStep::Leave;
        }
        if !self.hidden_by_us {
            return PartnerHideStep::Leave;
        }
        self.hidden_by_us = false;
        if executes && !drawn {
            PartnerHideStep::ShowAgain
        } else {
            PartnerHideStep::Leave
        }
    }
}

/// Whether the AI partner Kaine should be hidden: someone on this link already is Kaine.
pub fn partner_kaine_hidden_wanted(own: Option<PartnerBody>, remote: Option<PartnerBody>) -> bool {
    [own, remote].contains(&Some(PartnerBody::SeedKaineE))
}
