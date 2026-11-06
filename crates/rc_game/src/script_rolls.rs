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
pub const ANY_PACK_HASH: u64 = 0;
pub const ROLL_ALLOW_LIST: [(u64, u64); 7] = [
    (ANY_PACK_HASH, 0xF942_280A_1514_6DF7),
    (ANY_PACK_HASH, 0xF942_290A_1514_6FAA),
    (ANY_PACK_HASH, 0xD608_4132_6CD3_AD62),
    (ANY_PACK_HASH, 0xCE57_6E6B_8F0A_F230),
    (ANY_PACK_HASH, 0x3310_20B4_F933_5513),
    (0x7FC3_DDA4_3F9E_64CE, 0xCE0E_ECAD_70F2_71E9),
    (0xC6C9_9BCA_24C0_1AAD, 0x3CD8_C38F_0726_42CB),
];
pub const GATHERING_ROLL_FUNCTION_HASHES: [u64; 3] = [
    0x420F_FD1C_A89C_FF66,
    0xFAD1_4D85_9A87_9F54,
    0x3998_1B93_BB99_C492,
];
