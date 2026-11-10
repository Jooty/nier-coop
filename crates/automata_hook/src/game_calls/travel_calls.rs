use automata_game::address::{FAST_TRAVEL_OBJECT, FAST_TRAVEL_TO_POINT_FUNCTION};
use automata_game::travel::{
    FAST_TRAVEL_POINT_LIST_HEAD_OFFSET, FAST_TRAVEL_STATE_OFFSET, FAST_TRAVEL_STEP_OFFSET,
    INTERNED_NAME_TEXT_POINTER_OFFSET, MAX_TRAVEL_POINTS_WALKED, TRAVEL_POINT_CODE_NAME_OFFSET,
    TRAVEL_POINT_LIST_END_MARK, TRAVEL_POINT_NAME_READ_LIMIT, TRAVEL_POINT_NEXT_OFFSET,
};

use crate::game_calls::GameCalls;

type FastTravelToPoint = unsafe extern "system" fn(usize, *const u64) -> usize;

impl GameCalls {
    /// Asks the game's own transport for the point with this code. False when the game could not
    /// intern the name or its transport is not idle; the jump itself happens in later frames.
    #[expect(unsafe_code, reason = "calls the game's fast travel entry")]
    pub fn fast_travel_to_point(&self, code: &str) -> bool {
        self.with_interned_name(code, |interned| {
            if *interned == 0 {
                return false;
            }
            // SAFETY: the addresses are the fast travel entry and its object in the verified
            // build; the entry copies the interned name it is given a pointer to. That the
            // frame-end hook may call it is a guess until run.
            let taken = unsafe {
                core::mem::transmute::<usize, FastTravelToPoint>(
                    self.address_of(FAST_TRAVEL_TO_POINT_FUNCTION),
                )(self.address_of(FAST_TRAVEL_OBJECT), interned)
            };
            taken & 0xFF != 0
        })
    }

    pub fn fast_travel_state(&self) -> Option<u32> {
        self.read_u32_at(
            self.address_value(FAST_TRAVEL_OBJECT) + u64::from(FAST_TRAVEL_STATE_OFFSET),
        )
        .ok()
    }

    pub fn fast_travel_step(&self) -> Option<i32> {
        self.read_i32_at(
            self.address_value(FAST_TRAVEL_OBJECT) + u64::from(FAST_TRAVEL_STEP_OFFSET),
        )
        .ok()
    }

    /// The code names of the travel points the game holds now, in list order.
    pub fn travel_point_names(&self) -> Vec<String> {
        let head =
            self.address_value(FAST_TRAVEL_OBJECT) + u64::from(FAST_TRAVEL_POINT_LIST_HEAD_OFFSET);
        let mut names = Vec::new();
        let mut point = self.read_u64_at(head).unwrap_or(0);
        for _ in 0..MAX_TRAVEL_POINTS_WALKED {
            if point == 0 || point == TRAVEL_POINT_LIST_END_MARK {
                break;
            }
            names.extend(self.travel_point_name(point));
            point = self
                .read_u64_at(point + u64::from(TRAVEL_POINT_NEXT_OFFSET))
                .unwrap_or(0);
        }
        names
    }

    fn travel_point_name(&self, point: u64) -> Option<String> {
        let nonzero = |address: u64| self.read_u64_at(address).ok().filter(|value| *value != 0);
        let interned = nonzero(point + u64::from(TRAVEL_POINT_CODE_NAME_OFFSET))?;
        let text = nonzero(interned + u64::from(INTERNED_NAME_TEXT_POINTER_OFFSET))?;
        let name = self.read_c_string_at(text, TRAVEL_POINT_NAME_READ_LIMIT);
        (!name.is_empty() && name.bytes().all(|byte| byte.is_ascii_graphic())).then_some(name)
    }
}
