/// Why a copy counts as unattended for a skip vote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unattended {
    WindowUnfocused,
    NotTheForegroundWindow,
    PauseMenuOpen,
}

impl Unattended {
    /// The first reason that holds, in the order unfocused, not foreground, pause menu.
    pub fn of(window_unfocused: bool, foreground: bool, pause_menu_open: bool) -> Option<Self> {
        if window_unfocused {
            Some(Self::WindowUnfocused)
        } else if !foreground {
            Some(Self::NotTheForegroundWindow)
        } else if pause_menu_open {
            Some(Self::PauseMenuOpen)
        } else {
            None
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::WindowUnfocused => "its window is unfocused",
            Self::NotTheForegroundWindow => "it is not the foreground window",
            Self::PauseMenuOpen => "its pause menu is open",
        }
    }
}
