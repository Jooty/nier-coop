use crate::npcs::NPC_TALK_COMMAND_IDS;

pub fn refuses_talk_start(command_id: i32, partner_is_talking_to_it: bool) -> bool {
    partner_is_talking_to_it && NPC_TALK_COMMAND_IDS.contains(&command_id)
}

pub fn talk_busy_notice(partner_name: &str) -> String {
    format!("{partner_name} is talking to them")
}
