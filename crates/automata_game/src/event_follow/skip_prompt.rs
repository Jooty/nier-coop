/// The line a skip vote shows on this copy's screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipPrompt {
    PartnerWantsToSkip,
    WaitingForThePartner,
}

impl SkipPrompt {
    pub fn text(self, partner_name: &str) -> String {
        match self {
            Self::PartnerWantsToSkip => format!("{partner_name} wants to skip this cutscene"),
            Self::WaitingForThePartner => format!("You want to skip; waiting for {partner_name}"),
        }
    }
}
