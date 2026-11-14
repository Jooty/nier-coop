use nt_wire::MAX_SESSION_SLOTS;

pub const ROSTER_COLUMN_SEATS: usize = 8;

/// Who sits in one seat of the free roam board: you, the member at that index of the members list, or nobody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterSeat {
    Own,
    Member(usize),
    Open,
}

/// The free roam board: its seats in slot order, one column of eight up to eight players and two beyond.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roster {
    pub columns: usize,
    pub seats: Vec<RosterSeat>,
}

/// Lays out the board with every player at its own slot, so a seat's number is its slot plus one and a
/// slot left by a player stays open. The second column opens once a slot past the first eight is taken.
pub fn roster(member_slots: &[u8], own_slot: u8) -> Roster {
    let taken = member_slots
        .iter()
        .enumerate()
        .map(|(index, slot)| (*slot, RosterSeat::Member(index)))
        .chain(std::iter::once((own_slot, RosterSeat::Own)))
        .filter(|(slot, _)| *slot < MAX_SESSION_SLOTS);
    let mut seats = vec![RosterSeat::Open; usize::from(MAX_SESSION_SLOTS)];
    let mut highest = 0;
    for (slot, seat) in taken {
        seats[usize::from(slot)] = seat;
        highest = highest.max(usize::from(slot));
    }
    let columns = if highest < ROSTER_COLUMN_SEATS { 1 } else { 2 };
    seats.truncate(columns * ROSTER_COLUMN_SEATS);
    Roster { columns, seats }
}
