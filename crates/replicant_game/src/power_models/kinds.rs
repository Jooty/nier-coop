use replicant_proto::PowerModelKind;

pub const POWER_MODEL_KINDS: [PowerModelKind; 4] = [
    PowerModelKind::BlackHand030,
    PowerModelKind::BlackHand020,
    PowerModelKind::BlackHandAlt030,
    PowerModelKind::BlackHandAlt020,
];

pub fn power_model_pack_name(kind: PowerModelKind) -> &'static str {
    match kind {
        PowerModelKind::BlackHand030 => "blackhand030",
        PowerModelKind::BlackHand020 => "blackhand020",
        PowerModelKind::BlackHandAlt030 => "d_blackhand030",
        PowerModelKind::BlackHandAlt020 => "d_blackhand020",
    }
}
