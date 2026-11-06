use serde::{Deserialize, Serialize};

pub const NO_MOTION_NAME_HASH: u32 = 0;
const FNV_OFFSET_BASIS: u32 = 0x811C_9DC5;
const FNV_PRIME: u32 = 0x0100_0193;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartnerBody {
    #[default]
    #[serde(rename = "nier")]
    Nier,
    #[serde(rename = "prologue")]
    PrologueCharacter,
}

pub fn motion_name_hash(name: &str) -> u32 {
    let hash = name.bytes().fold(FNV_OFFSET_BASIS, |hash, byte| {
        hash.wrapping_mul(FNV_PRIME) ^ u32::from(byte)
    });
    if hash == NO_MOTION_NAME_HASH { 1 } else { hash }
}
