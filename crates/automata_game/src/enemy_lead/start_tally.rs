use std::collections::HashMap;

use crate::enemies::EnemyId;

/// The host's count of state starts per shared enemy. A start the state start hook saw counts
/// with the arguments it was made with; without one, a change of the state id read from the
/// enemy counts as a start of what was read (a class whose own state start the hook misses).
#[derive(Debug, Clone, Default)]
pub struct StartTally {
    enemies: HashMap<EnemyId, Tallied>,
}

/// A state start as the state start hook saw it: how many it has seen for that body, and the
/// arguments of the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HookedStarts {
    pub count: u32,
    pub last: [i32; 4],
}

#[derive(Debug, Clone, Copy)]
struct Tallied {
    state_id_read: i32,
    hooked_count: Option<u32>,
    starts: u32,
    sent: [i32; 4],
}

impl StartTally {
    /// The start count and the state to send for one enemy this round.
    pub fn tally(
        &mut self,
        id: EnemyId,
        read: [i32; 4],
        hooked: Option<HookedStarts>,
    ) -> (u32, [i32; 4]) {
        let Some(tallied) = self.enemies.get_mut(&id) else {
            let sent = hooked.map_or(read, |hooked| hooked.last);
            self.enemies.insert(
                id,
                Tallied {
                    state_id_read: read[0],
                    hooked_count: hooked.map(|hooked| hooked.count),
                    starts: 0,
                    sent,
                },
            );
            return (0, sent);
        };
        let hooked_now = hooked.filter(|hooked| Some(hooked.count) != tallied.hooked_count);
        if let Some(hooked) = hooked_now {
            let seen = tallied
                .hooked_count
                .map_or(1, |before| hooked.count.wrapping_sub(before));
            tallied.starts = tallied.starts.wrapping_add(seen);
            tallied.sent = hooked.last;
        } else if read[0] != tallied.state_id_read {
            tallied.starts = tallied.starts.wrapping_add(1);
            tallied.sent = read;
        }
        tallied.state_id_read = read[0];
        tallied.hooked_count = hooked.map(|hooked| hooked.count).or(tallied.hooked_count);
        (tallied.starts, tallied.sent)
    }

    pub fn keep_only(&mut self, ids: &[EnemyId]) {
        self.enemies.retain(|id, _| ids.contains(id));
    }

    pub fn forget(&mut self) {
        self.enemies.clear();
    }
}
