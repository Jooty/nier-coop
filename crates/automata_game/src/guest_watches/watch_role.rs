use crate::travel::JumpRole;

/// What watching makes of this copy this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WatchRole {
    /// Watching does not apply: everything is as without the stage.
    #[default]
    Neither,
    /// The guest: its own hero is driven as the host's body; no partner body.
    Watches,
    /// The host: the guest watches it; no body is made or kept for the guest.
    IsWatched,
}

impl WatchRole {
    pub const fn as_number(self) -> u8 {
        match self {
            Self::Neither => 0,
            Self::Watches => 1,
            Self::IsWatched => 2,
        }
    }

    pub const fn from_number(number: u8) -> Self {
        match number {
            1 => Self::Watches,
            2 => Self::IsWatched,
            _ => Self::Neither,
        }
    }

    /// True on both copies while the guest watches: neither makes a body for the other.
    pub fn has_no_partner_body(self) -> bool {
        self != Self::Neither
    }
}

/// The host is watched while its own setting is on and the guest's state says it watches.
pub fn host_is_watched(setting_on: bool, role: JumpRole, guest_says_it_watches: bool) -> bool {
    setting_on && role == JumpRole::Host && guest_says_it_watches
}

/// The guest's machine applies only to a campaign guest with its host present.
pub fn guest_watch_applies(setting_on: bool, role: JumpRole, host_present: bool) -> bool {
    setting_on && role == JumpRole::Guest && host_present
}
