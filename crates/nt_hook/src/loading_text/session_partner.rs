#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPartner<'a> {
    NoSession,
    NoPartnerYet,
    Named(&'a str),
}

impl<'a> SessionPartner<'a> {
    pub fn from_session(in_session: bool, partner_name: Option<&'a str>) -> Self {
        match (in_session, partner_name) {
            (false, _) => Self::NoSession,
            (true, Some(name)) if !name.trim().is_empty() => Self::Named(name),
            (true, _) => Self::NoPartnerYet,
        }
    }
}
