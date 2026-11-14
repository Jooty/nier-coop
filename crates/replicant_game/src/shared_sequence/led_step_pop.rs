/// What a game does with a step only the host's guide reaches, at the moment it pops it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedStepPop {
    HostRunsAndSends,
    HostRunsAgain,
    /// The host's copy: it runs past the checks that drop a game's own copy, so whichever of the
    /// two relays (shared function, story event) delivers it first is the one that runs.
    GuestRunsTheHostsCopy,
    GuestDropsASecondCopyOfTheHosts,
    GuestDropsItsOwnCopy,
    GuestRunsItsOwn,
}

impl LedStepPop {
    pub const fn drops(self) -> bool {
        matches!(
            self,
            Self::GuestDropsASecondCopyOfTheHosts | Self::GuestDropsItsOwnCopy
        )
    }

    pub const fn passes_the_duplicate_checks(self) -> bool {
        matches!(self, Self::GuestRunsTheHostsCopy)
    }
}

pub const fn led_step_pop(host: bool, queued_by_the_mod: bool, ran_here: bool) -> LedStepPop {
    match (host, queued_by_the_mod, ran_here) {
        (true, _, false) => LedStepPop::HostRunsAndSends,
        (true, _, true) => LedStepPop::HostRunsAgain,
        (false, true, false) => LedStepPop::GuestRunsTheHostsCopy,
        (false, true, true) => LedStepPop::GuestDropsASecondCopyOfTheHosts,
        (false, false, true) => LedStepPop::GuestDropsItsOwnCopy,
        (false, false, false) => LedStepPop::GuestRunsItsOwn,
    }
}
