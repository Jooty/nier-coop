use crate::move_log::STATE_ACTION_MOVE_RVA;

const STATE_NAMES: [(u32, &str); 13] = [
    (STATE_ACTION_MOVE_RVA, "StateActionMove"),
    (0xC2_DE10, "StateActionAttack"),
    (0xC3_1298, "StateActionGuard"),
    (0xC3_1368, "StateActionJustGuard"),
    (0xC3_0028, "StateActionDodge"),
    (0xC3_00F8, "StateActionRoundDodge"),
    (0xC8_E2F8, "StateActionMagicCharge"),
    (0xC8_CFC8, "StateActionDamage"),
    (0xC8_E000, "StateActionChangeWeapon"),
    (0xC8_9ED8, "StateActionGlobal"),
    (0xC2_DC78, "StateActionSequential"),
    (0xC2_DD38, "StateActionSequentialBasic"),
    (0xC2_DEE0, "StateActionCriticalAttack"),
];

pub fn state_name_for_rva(rva: u32) -> &'static str {
    STATE_NAMES
        .iter()
        .find(|(known, _)| *known == rva)
        .map_or("unknown", |(_, name)| name)
}

pub fn action_set_for_seed(seed_name: &str) -> &'static str {
    match seed_name {
        "nierB" => "PlayerBoy",
        "nierT" | "nierY" | "nierF" => "PlayerDefault",
        "kaineE" => "PlayerKaine",
        _ => "unknown",
    }
}
