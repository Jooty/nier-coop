#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveVerdict {
    Follow,
    KeepHostsAndAnswer,
    KeepOwnPush,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakVerdict {
    BreakHere,
    WaitUntilHittable,
    NotHere,
}

/// The host's copy is the object: the guest follows every host move, and the host follows a
/// guest move unless the host is pushing or carrying that object itself. A guest that pushes an
/// object the host does not hold keeps its push: the host's resend of a resting place is older.
pub fn partner_move_verdict(
    i_am_host: bool,
    i_hold_it: bool,
    partner_holds_it: bool,
) -> MoveVerdict {
    match (i_am_host, i_hold_it, partner_holds_it) {
        (true, true, _) => MoveVerdict::KeepHostsAndAnswer,
        (false, true, false) => MoveVerdict::KeepOwnPush,
        _ => MoveVerdict::Follow,
    }
}

pub fn partner_break_verdict(found_here: bool, hittable_here: bool) -> BreakVerdict {
    match (found_here, hittable_here) {
        (false, _) => BreakVerdict::NotHere,
        (true, false) => BreakVerdict::WaitUntilHittable,
        (true, true) => BreakVerdict::BreakHere,
    }
}
