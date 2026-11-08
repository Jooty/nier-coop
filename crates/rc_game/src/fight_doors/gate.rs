use crate::fight_doors::{FIGHT_DOOR_PARTNER_RADIUS_UNITS, FIGHT_DOORS};
use crate::talk_guard::{PressAsk, PressVerdict, judge_press};
use crate::zone_rules::ZoneRadius;

pub fn is_fight_door(phase: &str, door_id: i32) -> bool {
    FIGHT_DOORS
        .iter()
        .any(|&(zone, door)| zone == phase && door == door_id)
}

pub fn judge_fight_door_press(phase: &str, door_id: i32, ask: &PressAsk<'_>) -> PressVerdict {
    if !is_fight_door(phase, door_id) {
        return PressVerdict::Allow;
    }
    judge_press(&PressAsk {
        radius: ZoneRadius::new(FIGHT_DOOR_PARTNER_RADIUS_UNITS),
        ..ask.clone()
    })
}
