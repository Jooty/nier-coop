use std::sync::Mutex;

use replicant_game::rule_rooms::{RoomClock, RuleRoomWatch};

pub(super) static WATCH: Mutex<RuleRoomWatch> = Mutex::new(RuleRoomWatch::new());
pub(super) static CLOCK: Mutex<RoomClock> = Mutex::new(RoomClock::new());
