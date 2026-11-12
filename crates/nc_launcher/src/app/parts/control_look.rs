/// How a control is drawn and whether it reacts: editable in sand ink, read-only in muted ink (a guest seeing
/// the host's choice), or unavailable in grey.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlLook {
    Editable,
    ReadOnly,
    Unavailable,
}

impl ControlLook {
    pub fn editable(self) -> bool {
        self == Self::Editable
    }
}
