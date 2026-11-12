use nc_wire::MAX_SESSION_SLOTS;

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

/// Lays out the board from the other members' slots. The host holds slot 0; a guest is not told its own
/// slot, so it is taken as the lowest slot no other member holds.
pub fn roster(member_slots: &[u8], viewer_is_guest: bool) -> Roster {
    let own_slot = if viewer_is_guest {
        (1..MAX_SESSION_SLOTS)
            .find(|slot| !member_slots.contains(slot))
            .unwrap_or(MAX_SESSION_SLOTS)
    } else {
        0
    };
    let mut taken: Vec<(u8, RosterSeat)> = member_slots
        .iter()
        .enumerate()
        .map(|(index, slot)| (*slot, RosterSeat::Member(index)))
        .collect();
    taken.push((own_slot, RosterSeat::Own));
    taken.sort_by_key(|(slot, _)| *slot);
    let party = taken.len();
    let columns = if party <= ROSTER_COLUMN_SEATS { 1 } else { 2 };
    let mut seats: Vec<RosterSeat> = taken.into_iter().map(|(_, seat)| seat).collect();
    seats.truncate(usize::from(MAX_SESSION_SLOTS));
    seats.resize(columns * ROSTER_COLUMN_SEATS, RosterSeat::Open);
    Roster { columns, seats }
}
