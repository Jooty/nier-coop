#[rustfmt::skip]
pub static STEPS_ONLY_THE_HOSTS_GUIDE_REACHES: [(&str, &str); 41] = [
    ("A_DESERT_TOWN_01", "EID_1410_a0070g_MiniGame"),
    ("A_DESERT_TOWN_01", "EID_1410_a0070g_North_04"),
    ("A_DESERT_TOWN_01", "EID_1410_a0070g_South_04"),
    ("A_DESERT_TOWN_01", "EID_1410_a0075g_01"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_02_Motion02"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_03"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_03_01"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_04"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_04_Motion02"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_05"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_05_01"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_08_Motion02"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_09"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_09_01"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_10"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_10_Motion02"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_10_Motion03"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_14_Motion02"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_15"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_15_01"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_16"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_16_Motion02"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_16_Motion03"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_20_Motion02"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_21"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_21_01"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_21_03"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_25_Motion02"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_26"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_26_01"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_27"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_27_Motion02"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_27_Motion03"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_30_Motion02"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_31"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_31_01"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_32"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_32_Motion02"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_32_Motion03"),
    ("A_DESERT_TOWN_01", "EID_1410_a0100g_34_NextTrap"),
    ("A_DESERT_TOWN_01", "MiniGame_End"),
];

/// A step of a sequence both players are in that the guest's game never reaches on its own: a
/// callback of a guide actor the guest's game shows as driven from the host. The host sends it
/// when its game pops it, and the guest runs it at once.
pub fn is_led_by_the_host(phase_label: &str, name: &str) -> bool {
    STEPS_ONLY_THE_HOSTS_GUIDE_REACHES
        .binary_search(&(phase_label, name))
        .is_ok()
}
