use crate::address::GameAddress;

pub const ENTRY_POINT: GameAddress = GameAddress::new(0x9D_6444, 0x9C_DB64);

pub const DIRECT_INPUT8_CREATE_IMPORT_SLOT: GameAddress = GameAddress::new(0xC1_B000, 0xC1_1000);

pub const FRAME_END_FUNCTION: GameAddress = GameAddress::new(0x24_A250, 0x24_37D0);

pub const INPUT_UPDATE_ALL_FUNCTION: GameAddress = GameAddress::new(0x86_C310, 0x86_3BB0);

pub const PAD_INPUT_RECORDS: GameAddress = GameAddress::new(0x13F_CEA0, 0x13F_0D20);
