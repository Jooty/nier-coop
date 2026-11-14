/// Why an event started on this copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartOrigin {
    /// This copy's story script asked for it.
    Script,
    /// The mod started it by its DataId, for the partner.
    ForThePartner,
    /// The script asked again for an event the mod already started here: it is skipped at once.
    Replay,
    /// Neither the story script nor the mod (an event chained to another, travel): mirrored too.
    Other,
}

impl StartOrigin {
    pub fn is_mirrored(self) -> bool {
        self != Self::Replay
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Script => "by this game's story script",
            Self::ForThePartner => "by its DataId for the partner",
            Self::Replay => {
                "by this game's story script again after it was started for the partner"
            }
            Self::Other => "by something other than the story script",
        }
    }
}
