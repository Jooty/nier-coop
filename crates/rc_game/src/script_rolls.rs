mod eligibility;
mod ordinals;
mod range;
mod store;

pub use eligibility::{RollFacts, roll_is_shared};
pub use ordinals::RollOrdinals;
pub use range::{RollNumber, number_from_slot, value_fits};
pub use store::SharedRollStore;

pub const GET_RAND_I_RVA: u32 = 0x6E_BE80;
pub const GET_RAND_F_RVA: u32 = 0x6E_BDC0;
pub const LUA_CALL_ARGUMENTS_OFFSET: u64 = 0x10;
pub const LUA_CALL_RESULTS_OFFSET: u64 = 0xEC0;
pub const SHARED_ROLL_EXPIRY_MS: u64 = 10_000;
pub const SHARED_ROLL_STORE_MAX_ENTRIES: usize = 256;
pub const ROLL_ALLOW_LIST: [(&str, &str); 7] = [
    ("", "KILL_KAINE1"),
    ("", "KILL_KAINE2"),
    ("", "Kaine_Respawn_Start_NR"),
    ("", "SubQuest_StartStage"),
    ("", "SET_VILLAGER_ANIMAL"),
    ("B_DESERT_FIELD_01", "load"),
    ("X_SOUTH_UNDERGROUND_021", "StartStage"),
];
pub const GATHERING_ROLL_FUNCTIONS: [&str; 3] = ["Sozai_Get", "SetSozai_Table", "Set_ItemMaterial"];
