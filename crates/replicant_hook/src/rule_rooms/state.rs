use std::sync::Mutex;

use replicant_game::rule_rooms::RuleRoomWatch;

pub(super) static WATCH: Mutex<RuleRoomWatch> = Mutex::new(RuleRoomWatch::new());
